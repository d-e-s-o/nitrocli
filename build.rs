// Copyright (C) 2021-2023 The Nitrocli Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::Result;

use grev::git_revision_auto;

fn main() -> Result<()> {
  let directory = env!("CARGO_MANIFEST_DIR");
  if let Some(git_revision) = git_revision_auto(directory)? {
    println!("cargo:rustc-env=NITROCLI_GIT_REVISION={}", git_revision);
  }
  Ok(())
}
