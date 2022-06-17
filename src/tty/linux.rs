// linux.rs

// Copyright (C) 2022 The Nitrocli Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use std::fmt;
use std::fs;
use std::io;
use std::io::BufRead as _;
use std::os::unix::io::AsRawFd as _;
use std::os::unix::io::RawFd;
use std::path;
use std::str::FromStr as _;

use anyhow::Context as _;

/// The prefix used in a `/proc/<pid>/status` file line indicating the
/// line containing the parent PID.
const PROC_PARENT_PID_PREFIX: &str = "PPid:";

/// An enumeration representing the `<process>` path component in
/// `/proc/<process>/`.
enum Process {
  Current,
  Pid(u32),
}

impl fmt::Display for Process {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Current => write!(f, "self"),
      Self::Pid(pid) => write!(f, "{}", pid),
    }
  }
}

/// Find the parent of a process.
fn find_parent(process: &Process) -> anyhow::Result<Process> {
  let status_path = format!("/proc/{}/status", process);
  // TODO: Use `File::options` once we bumped the minimum supported Rust
  //       version to 1.58.
  let file = fs::OpenOptions::new()
    .write(false)
    .read(true)
    .create(false)
    .open(&status_path)
    .with_context(|| format!("Failed to open {}", status_path))?;
  let mut file = io::BufReader::new(file);
  let mut line = String::new();

  loop {
    let count = file.read_line(&mut line)?;
    if count == 0 {
      break Err(anyhow::anyhow!(
        "Status file {} ended unexpectedly",
        status_path
      ));
    }

    if let Some(line) = line.strip_prefix(PROC_PARENT_PID_PREFIX) {
      let line = line.trim();
      let pid = u32::from_str(line).with_context(|| {
        format!(
          "Encountered string '{}' cannot be parsed as a file descriptor",
          line
        )
      })?;
      break Ok(Process::Pid(pid));
    }
    line.clear();
  }
}

/// Check whether the file at the provided path actually represents a
/// TTY.
fn represents_tty(path: &path::Path) -> anyhow::Result<bool> {
  let file = fs::OpenOptions::new()
    .write(false)
    .read(true)
    .create(false)
    .open(&path)
    .with_context(|| format!("Failed to open file {}", path.display()))?;

  // We could evaluate `errno` on failure, but we do not actually care
  // why it's not a TTY.
  let rc = unsafe { libc::isatty(file.as_raw_fd()) };
  Ok(rc == 1)
}

/// Retrieve a path to a file descriptor in a process, if possible.
fn retrieve_fd_path(process: &Process, fd: RawFd) -> anyhow::Result<path::PathBuf> {
  let fd_path = format!("/proc/{}/fd/{}", process, fd);
  fs::read_link(&fd_path).with_context(|| format!("Failed to read symbolic link {}", fd_path))
}

/// Retrieve the path to the TTY used by a process.
fn retrieve_tty_impl(mut process: Process) -> anyhow::Result<path::PathBuf> {
  let stdin_fd = io::stdin().as_raw_fd();
  // We assume stdin to merely be the constant 0. That's an assumption
  // we apply to all processes (but can only check for the current one).
  debug_assert_eq!(stdin_fd, 0);

  loop {
    let path = retrieve_fd_path(&process, stdin_fd)?;
    if let Ok(true) = represents_tty(&path) {
      break Ok(path);
    }

    process = find_parent(&process)?;
    // Terminate our search once we reached the root process, which has
    // a parent PID of 0.
    if matches!(process, Process::Pid(pid) if pid == 0) {
      break Err(anyhow::anyhow!("Process has no TTY"));
    }
  }
}

/// Retrieve a path to the TTY used for stdin, if any.
pub(crate) fn retrieve_tty() -> anyhow::Result<path::PathBuf> {
  retrieve_tty_impl(Process::Current)
}

#[cfg(test)]
mod tests {
  use super::*;

  use std::process;

  /// Check that we can retrieve the path to the TTY used for stdin.
  #[test]
  fn tty_retrieval() {
    // We may be run with stdin not referring to a TTY in CI.
    if unsafe { libc::isatty(io::stdin().as_raw_fd()) } == 0 {
      return;
    }

    let tty = retrieve_tty().unwrap();
    // To check sanity of the reported path at least somewhat, we just
    // try opening the file, which should be possible. Note that we open
    // in write mode, because for one reason or another we would not
    // actually fail opening a *directory* in read-only mode.
    let _file = fs::OpenOptions::new()
      .create(false)
      .write(true)
      .read(true)
      .open(tty)
      .unwrap();
  }

  /// Check that we can properly retrieve the TTY via a parent process.
  #[test]
  fn parent_tty_retrieval() {
    // If *we* don't have a TTY readily available we are probably run in
    // CI and don't have permission to access the parent's TTY either.
    // We really can only skip the test then.
    if unsafe { libc::isatty(io::stdin().as_raw_fd()) } == 0 {
      return;
    }

    fn test(stdin: process::Stdio, redirection: &str) {
      let mut child = process::Command::new("sh")
        .stdin(stdin)
        // We need to read a line from stdout in order to find out the
        // (recursive) child's PID.
        .stdout(process::Stdio::piped())
        // We assume being run from the project root, which is what
        // `cargo` does. That may not be the case if the binary is
        // executed manually, though. That's unsupported.
        .arg("src/tty/tty.sh")
        .arg(redirection)
        .spawn()
        .unwrap();

      let mut line = String::new();
      let mut stdout = io::BufReader::new(child.stdout.as_mut().unwrap());
      let _ = stdout.read_line(&mut line).unwrap();
      let pid = u32::from_str(line.trim()).unwrap();

      let process = Process::Pid(pid);
      let tty = retrieve_tty_impl(process).unwrap();
      let _file = fs::OpenOptions::new()
        .create(false)
        .write(true)
        .read(true)
        .open(tty)
        .unwrap();

      // Clean up the child. Note that we could end up leaking the
      // processes earlier if any of the unwraps above fails. We made
      // the child terminate on its own after a while, though, instead
      // of increasing test complexity and decreasing debuggability by
      // handling all unwraps gracefully.
      let () = child.kill().unwrap();
    }

    test(process::Stdio::null(), "pipe");
    test(process::Stdio::null(), "devnull");
    test(process::Stdio::inherit(), "pipe");
    test(process::Stdio::inherit(), "devnull");
    test(process::Stdio::piped(), "pipe");
    test(process::Stdio::piped(), "devnull");
  }
}
