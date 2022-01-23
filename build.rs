// Copyright (C) 2021-2022 The Nitrocli Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use std::process::Command;

use anyhow::bail;
use anyhow::Context;
use anyhow::Result;

const GIT: &str = "git";

/// Format a git command with the given list of arguments as a string.
fn git_command(args: &[&str]) -> String {
  args.iter().fold(GIT.to_string(), |mut cmd, arg| {
    cmd += " ";
    cmd += arg;
    cmd
  })
}

/// Run git with the provided arguments and read the output it emits.
fn git_output(args: &[&str]) -> Result<String> {
  let git = Command::new(GIT)
    .args(args)
    .output()
    .with_context(|| format!("failed to run `{}`", git_command(args)))?;

  if !git.status.success() {
    let code = if let Some(code) = git.status.code() {
      format!(" ({})", code)
    } else {
      String::new()
    };

    bail!(
      "`{}` reported non-zero exit-status{}",
      git_command(args),
      code
    );
  }

  let output = String::from_utf8(git.stdout).with_context(|| {
    format!(
      "failed to read `{}` output as UTF-8 string",
      git_command(args)
    )
  })?;

  Ok(output)
}

/// Run git with the provided arguments and report the status of the
/// command.
fn git_run(args: &[&str]) -> Result<bool> {
  Command::new(GIT)
    .args(args)
    .status()
    .with_context(|| format!("failed to run `{}`", git_command(args)))
    .map(|status| status.success())
}

/// Retrieve a git revision identifier that either includes the tag we
/// are on or the shortened SHA-1. It also contains an indication
/// whether local changes were present.
fn git_revision() -> Result<Option<String>> {
  // As a first step we check whether we are in a git repository and
  // whether git is working to begin with. If not, we can't do much; yet
  // we still want to allow the build to continue, so we merely print a
  // warning and continue without a git revision. But once these checks
  // are through, we treat subsequent failures as unexpected and fatal.
  match git_run(&["rev-parse", "--git-dir"]) {
    Ok(true) => (),
    Ok(false) => {
      println!("cargo:warning=Not in a git repository; unable to embed git revision");
      return Ok(None);
    }
    Err(err) => {
      println!(
        "cargo:warning=Failed to invoke `git`; unable to embed git revision: {}",
        err
      );
      return Ok(None);
    }
  }

  let local_changes = git_output(&["status", "--porcelain", "--untracked-files=no"])?;
  let modified = !local_changes.is_empty();

  // If we are on a tag then just include the tag name. Otherwise use
  // the shortened SHA-1.
  let revision = if let Ok(tag) = git_output(&["describe", "--exact-match", "--tags", "HEAD"]) {
    tag
  } else {
    git_output(&["rev-parse", "--short", "HEAD"])?
  };
  let revision = format!("{}{}", revision.trim(), if modified { "+" } else { "" });
  Ok(Some(revision))
}

fn main() -> Result<()> {
  if let Some(git_revision) = git_revision()? {
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
