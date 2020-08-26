// extensions.rs

// Copyright (C) 2020 The Nitrocli Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use std::env;
use std::fs;

use super::*;

#[test]
fn resolve_extensions() -> anyhow::Result<()> {
  let dir1 = tempfile::tempdir()?;
  let dir2 = tempfile::tempdir()?;

  {
    let ext1_path = dir1.path().join("nitrocli-ext1");
    let ext2_path = dir1.path().join("nitrocli-ext2");
    let ext3_path = dir2.path().join("nitrocli-super-1337-extensions111one");
    let _ext1 = fs::File::create(&ext1_path)?;
    let _ext2 = fs::File::create(&ext2_path)?;
    let _ext3 = fs::File::create(&ext3_path)?;

    let path = env::join_paths(&[dir1.path(), dir2.path()])?;
    assert_eq!(
      crate::commands::resolve_extension(&path, ffi::OsStr::new("ext1"))?,
      ext1_path
    );
    assert_eq!(
      crate::commands::resolve_extension(&path, ffi::OsStr::new("ext2"))?,
      ext2_path
    );
    assert_eq!(
      crate::commands::resolve_extension(&path, ffi::OsStr::new("super-1337-extensions111one"))?,
      ext3_path
    );

    let err = crate::commands::resolve_extension(&ffi::OsStr::new(""), ffi::OsStr::new("ext1"))
      .unwrap_err();
    assert_eq!(err.to_string(), "Extension nitrocli-ext1 not found");
  }
  Ok(())
}
