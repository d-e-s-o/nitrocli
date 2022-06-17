// linux.rs

// Copyright (C) 2022 The Nitrocli Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use std::ffi;
use std::io;
use std::os::unix::ffi::OsStringExt as _;
use std::os::unix::io::AsRawFd as _;
use std::path;

/// Retrieve a path to the TTY used for stdin, if any.
///
/// This function works on a best effort basis and skips any advanced
/// error reporting, knowing that callers do not care.
pub(crate) fn retrieve_tty() -> Result<path::PathBuf, ()> {
  let fd = io::stdin().as_raw_fd();
  let fd_path = format!("/proc/self/fd/{}\0", fd);
  let fd_path = ffi::CStr::from_bytes_with_nul(fd_path.as_bytes()).unwrap();

  let mut buffer = Vec::<u8>::with_capacity(56);
  // SAFETY: We provide valid pointers, `fd_path` is NUL terminated, and
  //         the provided capacity reflects the actual length of the
  //         buffer.
  let rc = unsafe {
    libc::readlink(
      fd_path.as_ptr(),
      buffer.as_mut_ptr() as *mut libc::c_char,
      buffer.capacity(),
    )
  };
  if rc <= 0 {
    return Err(());
  }

  let rc = rc as usize;
  // If `readlink` filled the entire buffer we could have experienced
  // silent truncation. So we just bail out out of precaution.
  if rc == buffer.capacity() {
    return Err(());
  }

  // SAFETY: At this point we know that `readlink` has written `rc`
  //         bytes to `buffer`.
  unsafe { buffer.set_len(rc) };

  Ok(ffi::OsString::from_vec(buffer).into())
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
