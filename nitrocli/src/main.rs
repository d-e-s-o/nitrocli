// main.rs

// *************************************************************************
// * Copyright (C) 2017-2018 Daniel Mueller (deso@posteo.net)              *
// *                                                                       *
// * This program is free software: you can redistribute it and/or modify  *
// * it under the terms of the GNU General Public License as published by  *
// * the Free Software Foundation, either version 3 of the License, or     *
// * (at your option) any later version.                                   *
// *                                                                       *
// * This program is distributed in the hope that it will be useful,       *
// * but WITHOUT ANY WARRANTY; without even the implied warranty of        *
// * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the         *
// * GNU General Public License for more details.                          *
// *                                                                       *
// * You should have received a copy of the GNU General Public License     *
// * along with this program.  If not, see <http://www.gnu.org/licenses/>. *
// *************************************************************************

#![deny(
  dead_code,
  duplicate_associated_type_bindings,
  illegal_floating_point_literal_pattern,
  improper_ctypes,
  intra_doc_link_resolution_failure,
  late_bound_lifetime_arguments,
  missing_copy_implementations,
  missing_debug_implementations,
  missing_docs,
  no_mangle_generic_items,
  non_shorthand_field_patterns,
  overflowing_literals,
  path_statements,
  patterns_in_fns_without_body,
  plugin_as_library,
  private_in_public,
  proc_macro_derive_resolution_fallback,
  safe_packed_borrows,
  stable_features,
  trivial_bounds,
  trivial_numeric_casts,
  type_alias_bounds,
  tyvar_behind_raw_pointer,
  unconditional_recursion,
  unions_with_drop_fields,
  unreachable_code,
  unreachable_patterns,
  unstable_features,
  unstable_name_collisions,
  unused,
  unused_comparisons,
  unused_import_braces,
  unused_lifetimes,
  unused_qualifications,
  unused_results,
  where_clauses_object_safety,
  while_true
)]
#![warn(
  bad_style,
  future_incompatible,
  nonstandard_style,
  renamed_and_removed_lints,
  rust_2018_compatibility,
  rust_2018_idioms
)]

//! Nitrocli is a program providing a command line interface to certain
//! commands of the Nitrokey Storage device.

mod error;
mod pinentry;

use std::process;
use std::result;

use nitrokey;

use crate::error::Error;

type Result<T> = result::Result<T, Error>;

const PIN_TYPE: pinentry::PinType = pinentry::PinType::User;

/// Create an `error::Error` with an error message of the format `msg: err`.
fn get_error(msg: &str, err: &nitrokey::CommandError) -> Error {
  Error::Error(format!("{}: {:?}", msg, err))
}

/// Connect to a Nitrokey Storage device and return it.
fn get_storage_device() -> Result<nitrokey::Storage> {
  nitrokey::Storage::connect()
    .or_else(|_| Err(Error::Error("Nitrokey device not found".to_string())))
}

/// Return a string representation of the given volume status.
fn get_volume_status(status: &nitrokey::VolumeStatus) -> &'static str {
  if status.active {
    if status.read_only {
      "read-only"
    } else {
      "active"
    }
  } else {
    "inactive"
  }
}

/// Pretty print the response of a status command.
fn print_status(status: &nitrokey::StorageStatus) {
  // We omit displaying information about the smartcard here as this
  // program really is only about the SD card portion of the device.
  println!(
    r#"Status:
  SD card ID:        {id:#x}
  firmware version:  {fwv0}.{fwv1}
  firmware:          {fw}
  storage keys:      {sk}
  user retry count:  {urc}
  admin retry count: {arc}
  volumes:
    unencrypted:     {vu}
    encrypted:       {ve}
    hidden:          {vh}"#,
    id = status.serial_number_sd_card,
    fwv0 = status.firmware_version_major,
    fwv1 = status.firmware_version_minor,
    fw = if status.firmware_locked {
      "locked"
    } else {
      "unlocked"
    },
    sk = if status.stick_initialized {
      "created"
    } else {
      "not created"
    },
    urc = status.user_retry_count,
    arc = status.admin_retry_count,
    vu = get_volume_status(&status.unencrypted_volume),
    ve = get_volume_status(&status.encrypted_volume),
    vh = get_volume_status(&status.hidden_volume),
  );
}

/// Inquire the status of the nitrokey.
fn status() -> Result<()> {
  let status = get_storage_device()?
    .get_status()
    .map_err(|err| get_error("Getting Storage status failed", &err))?;

  print_status(&status);
  Ok(())
}

/// Open the encrypted volume on the nitrokey.
fn open() -> Result<()> {
  let device = get_storage_device()?;

  let mut retry = 3;
  let mut error_msg: Option<&str> = None;
  loop {
    // TODO: Rethink the usage of String::from_utf8_lossy here. We may
    //       not want to silently modify the password!
    let passphrase = pinentry::inquire_passphrase(PIN_TYPE, error_msg)?;
    let passphrase = String::from_utf8_lossy(&passphrase);
    match device.enable_encrypted_volume(&passphrase) {
      Ok(()) => return Ok(()),
      Err(err) => match err {
        nitrokey::CommandError::WrongPassword => {
          pinentry::clear_passphrase(PIN_TYPE)?;
          retry -= 1;

          if retry > 0 {
            error_msg = Some("Wrong password, please reenter");
            continue;
          }
          let error = "Opening encrypted volume failed: Wrong password";
          return Err(Error::Error(error.to_string()));
        }
        err => return Err(get_error("Opening encrypted volume failed", &err)),
      },
    };
  }
}

#[link(name = "c")]
extern "C" {
  fn sync();
}

/// Close the previously opened encrypted volume.
fn close() -> Result<()> {
  // Flush all filesystem caches to disk. We are mostly interested in
  // making sure that the encrypted volume on the nitrokey we are
  // about to close is not closed while not all data was written to
  // it.
  unsafe { sync() };

  get_storage_device()?
    .disable_encrypted_volume()
    .map_err(|err| get_error("Closing encrypted volume failed", &err))
}

/// Clear the PIN stored when opening the nitrokey's encrypted volume.
fn clear() -> Result<()> {
  pinentry::clear_passphrase(PIN_TYPE)
}

// A macro for generating a match of the different supported commands.
// Each supplied command is converted into a string and matched against.
macro_rules! commands {
  ( $str:expr, [ $( $command:expr), *] ) => {
    match &*$str.to_string() {
      $(
        stringify!($command) => {
          if let Err(err) = $command() {
            println!("{}", err);
            return 1
          }
          return 0
        },
      )*
      x => {
        println!("Invalid command: {}", x);
        println!("Available commands: {}", stringify!( $($command)* ));
        return 1
      },
    }
  }
}

fn run() -> i32 {
  let argv: Vec<String> = std::env::args().collect();
  if argv.len() != 2 {
    println!("Usage: {} <command>", argv[0]);
    return 1;
  }

  commands!(&argv[1], [status, open, close, clear]);
}

fn main() {
  process::exit(run());
}
