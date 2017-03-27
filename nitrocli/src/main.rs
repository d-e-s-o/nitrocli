// main.rs

// *************************************************************************
// * Copyright (C) 2017 Daniel Mueller (deso@posteo.net)                   *
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


#![deny(missing_docs)]

//! Nitrocli is a program providing a command line interface to certain
//! commands of the Nitrokey Storage device.

extern crate hid as libhid;


mod error;
mod nitrokey;

use error::Error;
use std::process;
use std::result;

type Result<T> = result::Result<T, Error>;
type NitroFunc = Fn(&mut libhid::Handle) -> Result<()>;


/// Find and open the nitrokey device and execute a function on it.
fn nitrokey_do(function: &NitroFunc) -> Result<()> {
  let hid = libhid::init()?;
  // The Manager::find method is plain stupid as it still returns an
  // iterable. Using it does not help in more concise error handling.
  for device in hid.devices() {
    if device.vendor_id() == nitrokey::VID && device.product_id() == nitrokey::PID {
      return function(&mut device.open()?);
    }
  }
  return Err(Error::Error("Nitrokey device not found".to_string()));
}


/// Open the encrypted volume on the nitrokey.
fn open() -> Result<()> {
  return nitrokey_do(&|handle| {
    println!("Found nitrokey. Opening encrypted volume...");
    return Ok(());
  });
}


/// Close the previously opened encrypted volume.
fn close() -> Result<()> {
  return nitrokey_do(&|handle| {
    println!("Found nitrokey. Closing encrypted volume...");
    return Ok(());
  });
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

  commands!(&argv[1], [open, close]);
}

fn main() {
  process::exit(run());
}
