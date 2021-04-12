// lib.rs

// Copyright (C) 2020-2021 The Nitrocli Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use std::env;
use std::ffi;
use std::path;
use std::process;

use anyhow::Context as _;

/// A context providing information relevant to `nitrocli` extensions.
#[derive(Debug)]
pub struct Context {
  /// Path to the `nitrocli` binary.
  nitrocli: ffi::OsString,
  /// The path to the USB device that `nitrocli` would connect to, if
  /// any.
  resolved_usb_path: Option<String>,
  /// The verbosity that `nitrocli` should use.
  verbosity: Option<u8>,
  /// The project directory root to use for the extension in question.
  project_dirs: directories::ProjectDirs,
}

impl Context {
  /// Create a new `Context` with information provided by `nitrocli`
  /// via environment variables.
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

  /// Retrieve `Nitrocli` object for invoking the main `nitrocli`
  /// program.
  pub fn nitrocli(&self) -> Nitrocli {
    Nitrocli::from_context(self)
  }

  /// Connect to a Nitrokey (or Librem Key) device as `nitrocli` would.
  pub fn connect<'mgr>(
    &self,
    mgr: &'mgr mut nitrokey::Manager,
  ) -> anyhow::Result<nitrokey::DeviceWrapper<'mgr>> {
    if let Some(usb_path) = &self.resolved_usb_path {
      mgr.connect_path(usb_path.to_owned()).map_err(From::from)
    } else {
      // TODO: Improve error message. Unfortunately, we can't easily
      //       determine whether we have no or more than one (matching)
      //       device.
      Err(anyhow::anyhow!("Could not connect to Nitrokey device"))
    }
  }

  /// Retrieve the path to the directory in which this extension may
  /// store cacheable artifacts.
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

/// A type allowing for convenient invocation of `nitrocli` itself.
#[derive(Debug)]
pub struct Nitrocli {
  cmd: process::Command,
}

impl Nitrocli {
  /// Create a new `Nitrocli` instance from a `Context`.
  fn from_context(ctx: &Context) -> Nitrocli {
    Self {
      cmd: process::Command::new(&ctx.nitrocli),
    }
  }

  /// Add an argument to the `nitrocli` invocation.
  pub fn arg<S>(&mut self, arg: S) -> &mut Nitrocli
  where
    S: AsRef<ffi::OsStr>,
  {
    self.cmd.arg(arg);
    self
  }

  /// Add multiple arguments to the `nitrocli` invocation.
  pub fn args<I, S>(&mut self, args: I) -> &mut Nitrocli
  where
    I: IntoIterator<Item = S>,
    S: AsRef<ffi::OsStr>,
  {
    self.cmd.args(args);
    self
  }

  /// Invoke `nitrocli` and retrieve its output as a string.
  ///
  /// Note that any error messages emitted by `nitrocli` will not be
  /// intercepted/captured but will directly be passed through. It is
  /// recommended that extensions terminate on failure.
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

  /// Invoke `nitrocli`.
  pub fn spawn(&mut self) -> anyhow::Result<()> {
    let mut child = self.cmd.spawn().context("Failed to invoke nitrocli")?;
    child.wait().context("Failed to wait on nitrocli")?;
    Ok(())
  }
}
