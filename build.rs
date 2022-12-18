// Copyright (C) 2021-2022 The Nitrocli Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use std::io::stdout;

use anyhow::Result;

use grev::get_revision as get_git_revision;

fn main() -> Result<()> {
  let directory = env!("CARGO_MANIFEST_DIR");
  if let Some(git_revision) = get_git_revision(directory, stdout().lock())? {
    println!("cargo:rustc-env=NITROCLI_GIT_REVISION={}", git_revision);
  }
  // Make sure to run this script again if any of our sources files or
  // any relevant version control files changes (e.g., when creating a
  // commit or a tag).
  println!("cargo:rerun-if-changed=.git/index");
  println!("cargo:rerun-if-changed=.git/refs/");
  println!("cargo:rerun-if-changed=Cargo.lock");
  println!("cargo:rerun-if-changed=Cargo.toml");
  println!("cargo:rerun-if-changed=ext/");
  println!("cargo:rerun-if-changed=src/");
  println!("cargo:rerun-if-changed=var/");
  Ok(())
}
