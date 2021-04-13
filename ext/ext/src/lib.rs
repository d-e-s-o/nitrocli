// lib.rs

// Copyright (C) 2020-2021 The Nitrocli Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use std::env;
use std::ffi;
use std::path;
use std::process;

use anyhow::Context as _;

#[derive(Debug)]
pub struct Context {
  nitrocli: ffi::OsString,
  resolved_usb_path: Option<String>,
  verbosity: Option<u8>,
  project_dirs: directories::ProjectDirs,
}

impl Context {
  pub fn from_env() -> anyhow::Result<Self> {
    let nitrocli = env::var_os("NITROCLI_BINARY")
      .context("NITROCLI_BINARY environment variable not present")
      .context("Failed to retrieve nitrocli path")?;

    let resolved_usb_path = env::var("NITROCLI_RESOLVED_USB_PATH").ok();

    let verbosity = env::var_os("NITROCLI_VERBOSITY")
      .context("NITROCLI_VERBOSITY environment variable not present")
      .context("Failed to retrieve nitrocli verbosity")?;
    let verbosity = if verbosity.len() == 0 {
      None
    } else {
      let verbosity = verbosity
        .to_str()
        .context("Provided verbosity string is not valid UTF-8")?;
      let verbosity = u8::from_str_radix(verbosity, 10).context("Failed to parse verbosity")?;
      set_log_level(verbosity);
      Some(verbosity)
    };

    let exe =
      env::current_exe().context("Failed to determine the path of the extension executable")?;
    let name = exe
      .file_name()
      .context("Failed to extract the name of the extension executable")?
      .to_str()
      .context("The name of the extension executable contains non-UTF-8 characters")?;
    let project_dirs = directories::ProjectDirs::from("", "", name).with_context(|| {
      format!(
        "Could not determine the application directories for the {} extension",
        name
      )
    })?;

    Ok(Self {
      nitrocli,
      resolved_usb_path,
      verbosity,
      project_dirs,
    })
  }

  pub fn nitrocli(&self) -> Nitrocli {
    Nitrocli::from_context(self)
  }

  pub fn connect<'mgr>(
    &self,
    mgr: &'mgr mut nitrokey::Manager,
  ) -> anyhow::Result<nitrokey::DeviceWrapper<'mgr>> {
    if let Some(usb_path) = &self.resolved_usb_path {
      mgr.connect_path(usb_path.to_owned()).map_err(From::from)
    } else {
      // TODO: Improve error message.  Unfortunately, we can’t easily determine whether we have no
      // or more than one (matching) device.
      Err(anyhow::anyhow!("Could not connect to Nitrokey device"))
    }
  }

  pub fn cache_dir(&self) -> &path::Path {
    self.project_dirs.cache_dir()
  }
}

// See src/command.rs in nitrocli core.
fn set_log_level(verbosity: u8) {
  let log_lvl = match verbosity {
    // The error log level is what libnitrokey uses by default. As such,
    // there is no harm in us setting that as well when the user did not
    // ask for higher verbosity.
    0 => nitrokey::LogLevel::Error,
    1 => nitrokey::LogLevel::Warning,
    2 => nitrokey::LogLevel::Info,
    3 => nitrokey::LogLevel::DebugL1,
    4 => nitrokey::LogLevel::Debug,
    _ => nitrokey::LogLevel::DebugL2,
  };
  nitrokey::set_log_level(log_lvl);
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
    let output = self.cmd.output().context("Failed to invoke nitrocli")?;
    // We want additional nitrocli emitted output to be visible to the
    // user (typically controlled through -v/--verbose below). Note that
    // this means that we will not be able to access this output for
    // error reporting purposes.
    self.cmd.stderr(process::Stdio::inherit());

    if output.status.success() {
      String::from_utf8(output.stdout).map_err(From::from)
    } else {
      Err(anyhow::anyhow!("nitrocli call failed"))
    }
  }

  pub fn spawn(&mut self) -> anyhow::Result<()> {
    let mut child = self.cmd.spawn().context("Failed to invoke nitrocli")?;
    child.wait().context("Failed to wait on nitrocli")?;
    Ok(())
  }
}
