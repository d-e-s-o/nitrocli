// ext.rs

// Copyright (C) 2020 The Nitrocli Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use std::env;
use std::ffi;
use std::fmt;
use std::process;
use std::str;

use anyhow::Context as _;

pub struct Context {
  /// The path to the nitrocli binary.
  pub nitrocli: ffi::OsString,
  /// The nitrokey model to use.
  pub model: nitrokey::Model,
  /// The verbosity level to use.
  pub verbosity: u8,
}

impl Context {
  pub fn from_env() -> anyhow::Result<Self> {
    let nitrocli = env::var_os("NITROCLI_BINARY")
      .context("NITROCLI_BINARY environment variable not present")
      .context("Failed to retrieve nitrocli path")?;

    let model = env::var_os("NITROCLI_MODEL")
      .context("NITROCLI_MODEL environment variable not present")
      .context("Failed to retrieve nitrocli model")?;
    let model = model
      .to_str()
      .context("Provided model string is not valid UTF-8")?;
    let model = match model {
      "pro" => nitrokey::Model::Pro,
      "storage" => nitrokey::Model::Storage,
      _ => anyhow::bail!("Provided model is not valid: '{}'", model),
    };

    let verbosity = env::var_os("NITROCLI_VERBOSITY")
      .context("NITROCLI_VERBOSITY environment variable not present")
      .context("Failed to retrieve nitrocli verbosity")?;
    let verbosity = verbosity
      .to_str()
      .context("Provided verbosity string is not valid UTF-8")?;
    let verbosity = u8::from_str_radix(verbosity, 10).context("Failed to parse verbosity")?;

    Ok(Self {
      nitrocli,
      model,
      verbosity,
    })
  }
}

#[derive(Debug)]
pub struct Nitrocli {
  cmd: process::Command,
}

impl Nitrocli {
  pub fn from_context(ctx: &Context) -> Nitrocli {
    Self {
      cmd: process::Command::new(&ctx.nitrocli),
    }
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
    if output.status.success() {
      String::from_utf8(output.stdout).map_err(From::from)
    } else {
      Err(anyhow::anyhow!(
        "nitrocli call failed: {}",
        String::from_utf8_lossy(&output.stderr)
      ))
    }
  }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum OtpAlgorithm {
  Hotp,
  Totp,
}

impl fmt::Display for OtpAlgorithm {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "{}",
      match self {
        OtpAlgorithm::Hotp => "hotp",
        OtpAlgorithm::Totp => "totp",
      }
    )
  }
}

impl str::FromStr for OtpAlgorithm {
  type Err = anyhow::Error;

  fn from_str(s: &str) -> Result<OtpAlgorithm, Self::Err> {
    match s {
      "hotp" => Ok(OtpAlgorithm::Hotp),
      "totp" => Ok(OtpAlgorithm::Totp),
      _ => Err(anyhow::anyhow!("Unexpected OTP algorithm: {}", s)),
    }
  }
}

impl<'de> serde::Deserialize<'de> for OtpAlgorithm {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    use serde::de::Error as _;

    str::FromStr::from_str(&String::deserialize(deserializer)?).map_err(D::Error::custom)
  }
}

impl serde::Serialize for OtpAlgorithm {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    self.to_string().serialize(serializer)
  }
}
