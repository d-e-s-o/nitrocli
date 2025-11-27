// Copyright (C) 2021-2025 The Nitrocli Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use std::env;

use anyhow::Context as _;
use anyhow::Result;

use grev::git_revision_auto;

fn main() -> Result<()> {
  let manifest_dir =
    env::var_os("CARGO_MANIFEST_DIR").context("CARGO_MANIFEST_DIR variable not set")?;
  if let Some(git_revision) = git_revision_auto(manifest_dir)? {
    println!("cargo:rustc-env=NITROCLI_GIT_REVISION={}", git_revision);
  }
  Ok(())
}
