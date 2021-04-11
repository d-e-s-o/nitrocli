// ext.rs

// Copyright (C) 2020-2021 The Nitrocli Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use std::env;
use std::ffi;
use std::process;

use anyhow::Context as _;

#[derive(Debug)]
pub struct Context {
  pub nitrocli: ffi::OsString,
  pub verbosity: Option<u8>,
  pub project_dirs: directories::ProjectDirs,
}

impl Context {
  pub fn from_env() -> anyhow::Result<Self> {
    let nitrocli = env::var_os("NITROCLI_BINARY")
      .context("NITROCLI_BINARY environment variable not present")
      .context("Failed to retrieve nitrocli path")?;

    let verbosity = env::var_os("NITROCLI_VERBOSITY")
      .context("NITROCLI_VERBOSITY environment variable not present")
      .context("Failed to retrieve nitrocli verbosity")?;
    let verbosity = if verbosity.len() == 0 {
      None
    } else {
      let verbosity = verbosity
        .to_str()
        .context("Provided verbosity string is not valid UTF-8")?;
      Some(u8::from_str_radix(verbosity, 10).context("Failed to parse verbosity")?)
    };

    let project_dirs = directories::ProjectDirs::from("", "", "nitrocli-cache")
      .context("Could not determine the nitrocli-cache application directories")?;

    Ok(Self {
      nitrocli,
      verbosity,
      project_dirs,
    })
  }
}

#[derive(Debug)]
pub struct Nitrocli {
  cmd: process::Command,
}

impl Nitrocli {
  pub fn from_context(ctx: &Context) -> Nitrocli {
    let mut cmd = process::Command::new(&ctx.nitrocli);
    if let Some(verbosity) = ctx.verbosity {
      for _ in 0..verbosity {
        cmd.arg("--verbose");
      }
    }
    Self { cmd }
  }

  pub fn arg(&mut self, arg: impl AsRef<ffi::OsStr>) -> &mut Nitrocli {
    self.cmd.arg(arg);
    self
  }

  pub fn args<I, S>(&mut self, args: I) -> &mut Nitrocli
  where
    I: IntoIterator<Item = S>,
    S: AsRef<ffi::OsStr>,
  {
    self.cmd.args(args);
    self
  }

  pub fn text(&mut self) -> anyhow::Result<String> {
    // TODO: inherit stderr from this process to show nitrocli debug messages
    let output = self.cmd.output().context("Failed to invoke nitrocli")?;
    if output.status.success() {
      String::from_utf8(output.stdout).map_err(From::from)
    } else {
      Err(anyhow::anyhow!(
        "nitrocli call failed: {}",
        String::from_utf8_lossy(&output.stderr)
      ))
    }
  }

  pub fn spawn(&mut self) -> anyhow::Result<()> {
    let mut child = self.cmd.spawn().context("Failed to invoke nitrocli")?;
    child.wait().context("Failed to wait on nitrocli")?;
    Ok(())
  }
}
