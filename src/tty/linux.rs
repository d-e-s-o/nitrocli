// linux.rs

// Copyright (C) 2022 The Nitrocli Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use std::fs;
use std::io;
use std::os::unix::io::AsRawFd as _;
use std::path;

/// Retrieve a path to the TTY used for stdin, if any.
///
/// This function works on a best effort basis and skips any advanced
/// error reporting, knowing that callers do not care.
pub(crate) fn retrieve_tty() -> Result<path::PathBuf, ()> {
  let fd = io::stdin().as_raw_fd();
  let fd_path = format!("/proc/self/fd/{}", fd);
  fs::read_link(fd_path).map_err(|_| ())
}

#[cfg(test)]
mod tests {
  use super::*;

  use std::fs;

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
}
