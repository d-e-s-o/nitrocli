// pinentry.rs

// Copyright (C) 2017-2022 The Nitrocli Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use std::borrow;
use std::ffi;
use std::fmt;
use std::process;
use std::str;

use anyhow::Context as _;

use crate::args;
use crate::Context;

type CowStr = borrow::Cow<'static, str>;

/// A trait representing a secret to be entered by the user.
pub trait SecretEntry: fmt::Debug {
  /// The cache ID to use for this secret.
  fn cache_id(&self) -> Option<CowStr>;
  /// The prompt to display when asking for the secret.
  fn prompt(&self) -> CowStr;
  /// The description to display when asking for the secret.
  fn description(&self, mode: Mode) -> CowStr;
  /// The minimum number of characters the secret needs to have.
  fn min_len(&self) -> u8;
}

#[derive(Debug)]
pub struct PinEntry {
  pin_type: args::PinType,
  model: nitrokey::Model,
  serial: nitrokey::SerialNumber,
}

impl PinEntry {
  pub fn from<'mgr, D>(pin_type: args::PinType, device: &D) -> anyhow::Result<Self>
  where
    D: nitrokey::Device<'mgr>,
  {
    let model = device.get_model();
    let serial = device
      .get_serial_number()
      .context("Failed to retrieve serial number")?;

    Ok(Self {
      pin_type,
      model,
      serial,
    })
  }

  pub fn pin_type(&self) -> args::PinType {
    self.pin_type
  }
}

impl SecretEntry for PinEntry {
  fn cache_id(&self) -> Option<CowStr> {
    let model = match self.model {
      nitrokey::Model::Librem => "librem",
      nitrokey::Model::Pro => "pro",
      nitrokey::Model::Storage => "storage",
      _ => "unknown",
    };
    let suffix = format!("{}:{}", model, self.serial);
    let cache_id = match self.pin_type {
      args::PinType::Admin => format!("nitrocli:admin:{}", suffix),
      args::PinType::User => format!("nitrocli:user:{}", suffix),
    };
    Some(cache_id.into())
  }

  fn prompt(&self) -> CowStr {
    match self.pin_type {
      args::PinType::Admin => "Admin PIN",
      args::PinType::User => "User PIN",
    }
    .into()
  }

  fn description(&self, mode: Mode) -> CowStr {
    format!(
      "{} for\r{} {}",
      match self.pin_type {
        args::PinType::Admin => match mode {
          Mode::Choose => "Please enter a new admin PIN",
          Mode::Confirm => "Please confirm the new admin PIN",
          Mode::Query => "Please enter the admin PIN",
        },
        args::PinType::User => match mode {
          Mode::Choose => "Please enter a new user PIN",
          Mode::Confirm => "Please confirm the new user PIN",
          Mode::Query => "Please enter the user PIN",
        },
      },
      self.model,
      self.serial,
    )
    .into()
  }

  fn min_len(&self) -> u8 {
    match self.pin_type {
      args::PinType::Admin => 8,
      args::PinType::User => 6,
    }
  }
}

#[derive(Debug)]
pub struct PwdEntry {
  model: nitrokey::Model,
  serial: nitrokey::SerialNumber,
}

impl PwdEntry {
  pub fn from<'mgr, D>(device: &D) -> anyhow::Result<Self>
  where
    D: nitrokey::Device<'mgr>,
  {
    let model = device.get_model();
    let serial = device
      .get_serial_number()
      .context("Failed to retrieve serial number")?;

    Ok(Self { model, serial })
  }
}

impl SecretEntry for PwdEntry {
  fn cache_id(&self) -> Option<CowStr> {
    None
  }

  fn prompt(&self) -> CowStr {
    "Password".into()
  }

  fn description(&self, mode: Mode) -> CowStr {
    format!(
      "{} for\r{} {}",
      match mode {
        Mode::Choose => "Please enter a new hidden volume password",
        Mode::Confirm => "Please confirm the new hidden volume password",
        Mode::Query => "Please enter a hidden volume password",
      },
      self.model,
      self.serial,
    )
    .into()
  }

  fn min_len(&self) -> u8 {
    // More or less arbitrary minimum length based on the fact that the
    // manual mentions six letter passwords in examples. Users
    // *probably* should go longer than that, but we don't want to be
    // too opinionated.
    6
  }
}

/// Secret entry mode for pinentry.
///
/// This enum describes the context of the pinentry query, for example
/// prompting for the current secret or requesting a new one. The mode
/// may affect the pinentry description and whether a quality bar is
/// shown.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mode {
  /// Let the user choose a new secret.
  Choose,
  /// Let the user confirm the previously chosen secret.
  Confirm,
  /// Query an existing secret.
  Query,
}

impl Mode {
  fn show_quality_bar(self) -> bool {
    self == Mode::Choose
  }
}

fn parse_pinentry_pin<R>(response: R) -> anyhow::Result<String>
where
  R: AsRef<str>,
{
  const DATA_PREFIX: &str = "D ";
  const ERR_PREFIX: &str = "ERR ";

  let string = response.as_ref();
  let lines: Vec<&str> = string.lines().collect();

  // We expect the response to be of the form:
  // > D passphrase
  // > OK
  // or potentially:
  // > ERR 83886179 Operation cancelled <Pinentry>
  //
  // Furthermore, in case of an empty password we'd get just an OK.
  match lines.as_slice() {
    ["OK"] => Ok(String::new()),
    [line, "OK"] if line.starts_with(DATA_PREFIX) => {
      let (_, pass) = line.split_at(DATA_PREFIX.len());
      Ok(pass.to_string())
    }
    [line] if line.starts_with(ERR_PREFIX) => {
      let (_, error) = line.split_at(ERR_PREFIX.len());
      anyhow::bail!("{}", error);
    }
    _ => anyhow::bail!("Unexpected response: {}", string),
  }
}

/// Connect to `gpg-agent`, run the provided command, and return the
/// output it emitted.
fn gpg_agent<C>(command: C) -> anyhow::Result<process::Output>
where
  C: AsRef<ffi::OsStr>,
{
  process::Command::new("gpg-connect-agent")
    .arg(command)
    .arg("/bye")
    .output()
    .context("Failed to invoke gpg-connect-agent")
}

/// Inquire a secret from the user.
///
/// This function inquires a secret from the user or returns a cached
/// entry, if available (and if caching is not disabled for the given
/// execution context). If an error message is set, it is displayed in
/// the entry dialog. The mode describes the context of the pinentry
/// dialog. It is used to choose an appropriate description and to
/// decide whether a quality bar is shown in the dialog.
pub fn inquire<E>(
  ctx: &mut Context<'_>,
  entry: &E,
  mode: Mode,
  error_msg: Option<&str>,
) -> anyhow::Result<String>
where
  E: SecretEntry,
{
  let cache_id = entry
    .cache_id()
    .and_then(|id| if ctx.config.no_cache { None } else { Some(id) })
    // "X" is a sentinel value indicating that no caching is desired.
    .unwrap_or_else(|| "X".into())
    .into();

  let error_msg = error_msg
    .map(|msg| msg.replace(" ", "+"))
    .unwrap_or_else(|| String::from("+"));
  let prompt = entry.prompt().replace(" ", "+");
  let description = entry.description(mode).replace(" ", "+");

  let mut command = "GET_PASSPHRASE --data ".to_string();
  if mode.show_quality_bar() {
    command += "--qualitybar ";
  }
  command += &[cache_id, error_msg, prompt, description].join(" ");

  // An error reported for the GET_PASSPHRASE command does not actually
  // cause gpg-connect-agent to exit with a non-zero error code, we have
  // to evaluate the output to determine success/failure.
  let output = gpg_agent(command)?;
  let response =
    str::from_utf8(&output.stdout).context("Failed to parse gpg-connect-agent output as UTF-8")?;
  parse_pinentry_pin(response).context("Failed to parse pinentry secret")
}

fn check<E>(entry: &E, secret: &str) -> anyhow::Result<()>
where
  E: SecretEntry,
{
  if secret.len() < usize::from(entry.min_len()) {
    anyhow::bail!(
      "The secret must be at least {} characters long",
      entry.min_len()
    )
  } else {
    Ok(())
  }
}

pub fn choose<E>(ctx: &mut Context<'_>, entry: &E) -> anyhow::Result<String>
where
  E: SecretEntry,
{
  clear(entry)?;
  let chosen = inquire(ctx, entry, Mode::Choose, None)?;
  clear(entry)?;
  check(entry, &chosen)?;

  let confirmed = inquire(ctx, entry, Mode::Confirm, None)?;
  clear(entry)?;

  if chosen != confirmed {
    anyhow::bail!("Entered secrets do not match")
  } else {
    Ok(chosen)
  }
}

fn parse_pinentry_response<R>(response: R) -> anyhow::Result<()>
where
  R: AsRef<str>,
{
  let string = response.as_ref();
  let lines = string.lines().collect::<Vec<_>>();

  if lines.len() == 1 && lines[0] == "OK" {
    // We got the only valid answer we accept.
    return Ok(());
  }
  anyhow::bail!("Unexpected response: {}", string)
}

/// Clear the cached secret represented by the given entry.
pub fn clear<E>(entry: &E) -> anyhow::Result<()>
where
  E: SecretEntry,
{
  if let Some(cache_id) = entry.cache_id() {
    let command = format!("CLEAR_PASSPHRASE {}", cache_id);
    let output = gpg_agent(command)?;
    let response = str::from_utf8(&output.stdout)
      .context("Failed to parse gpg-connect-agent output as UTF-8")?;

    parse_pinentry_response(response).context("Failed to parse pinentry response")
  } else {
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parse_pinentry_pin_empty() {
    let response = "OK\n";
    let expected = "";

    assert_eq!(parse_pinentry_pin(response).unwrap(), expected)
  }

  #[test]
  fn parse_pinentry_pin_good() {
    let response = "D passphrase\nOK\n";
    let expected = "passphrase";

    assert_eq!(parse_pinentry_pin(response).unwrap(), expected)
  }

  #[test]
  fn parse_pinentry_pin_error() {
    let error = "83886179 Operation cancelled";
    let response = "ERR ".to_string() + error + "\n";
    let expected = error;

    let error = parse_pinentry_pin(response).unwrap_err();
    assert_eq!(error.to_string(), expected)
  }

  #[test]
  fn parse_pinentry_pin_unexpected() {
    let response = "foobar\n";
    let expected = format!("Unexpected response: {}", response);
    let error = parse_pinentry_pin(response).unwrap_err();
    assert_eq!(error.to_string(), expected)
  }

  #[test]
  fn parse_pinentry_response_ok() {
    assert!(parse_pinentry_response("OK\n").is_ok())
  }

  #[test]
  fn parse_pinentry_response_ok_no_newline() {
    assert!(parse_pinentry_response("OK").is_ok())
  }

  #[test]
  fn parse_pinentry_response_unexpected() {
    let response = "ERR 42";
    let expected = format!("Unexpected response: {}", response);
    let error = parse_pinentry_response(response).unwrap_err();
    assert_eq!(error.to_string(), expected)
  }
}
