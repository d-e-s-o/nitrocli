// mod.rs

// *************************************************************************
// * Copyright (C) 2019 Daniel Mueller (deso@posteo.net)                   *
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

use std::fmt;

use nitrokey_test::test as test_device;

// TODO: This is a hack to make the nitrokey-test crate work across
//       module boundaries. Upon first use of the nitrokey_test::test
//       macro a new function, __nitrokey_mutex, will be emitted, but it
//       is not visible in a different module. To work around that we
//       trigger the macro here first and then `use super::*` from all
//       of the submodules.
#[test_device]
fn dummy() {}

mod run;
mod status;

/// An `Option<IntoArg>` that represents a non-present device. Rust can
/// be notoriously bad at inferring type parameters and this constant
/// alleviates the pain.
const NO_DEV: Option<nitrokey::Pro> = None;

/// A trait for conversion of a nitrokey::Device into an argument
/// representing the device model that the program recognizes.
pub trait IntoArg {
  fn into_arg(self) -> &'static str;
}

impl IntoArg for nitrokey::Pro {
  fn into_arg(self) -> &'static str {
    "--model=pro"
  }
}

impl IntoArg for nitrokey::Storage {
  fn into_arg(self) -> &'static str {
    "--model=storage"
  }
}

impl IntoArg for nitrokey::DeviceWrapper {
  fn into_arg(self) -> &'static str {
    match self {
      nitrokey::DeviceWrapper::Pro(x) => x.into_arg(),
      nitrokey::DeviceWrapper::Storage(x) => x.into_arg(),
    }
  }
}

/// A trait simplifying checking for expected errors.
pub trait UnwrapError {
  /// Unwrap an Error::Error variant.
  fn unwrap_str_err(self) -> String;
}

impl<T> UnwrapError for crate::Result<T>
where
  T: fmt::Debug,
{
  fn unwrap_str_err(self) -> String {
    match self.unwrap_err() {
      crate::error::Error::Error(err) => err,
      err => panic!("Unexpected error variant found: {:?}", err),
    }
  }
}

mod nitrocli {
  use super::*;

  use crate::args;
  use crate::Result;
  use crate::RunCtx;

  fn do_run<F, R, I>(device: Option<I>, args: &[&'static str], f: F) -> (R, Vec<u8>, Vec<u8>)
  where
    F: FnOnce(&mut RunCtx<'_>, Vec<String>) -> R,
    I: IntoArg,
  {
    let args = ["nitrocli"]
      .into_iter()
      .cloned()
      .chain(device.into_iter().map(IntoArg::into_arg))
      .chain(args.into_iter().cloned())
      .map(ToOwned::to_owned)
      .collect();

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();

    let ctx = &mut RunCtx {
      stdout: &mut stdout,
      stderr: &mut stderr,
    };

    (f(ctx, args), stdout, stderr)
  }

  /// Run `nitrocli`'s `run` function.
  pub fn run<I>(device: Option<I>, args: &[&'static str]) -> (i32, Vec<u8>, Vec<u8>)
  where
    I: IntoArg,
  {
    do_run(device, args, |c, a| crate::run(c, a))
  }

  /// Run `nitrocli`'s `handle_arguments` function.
  pub fn handle<I>(device: Option<I>, args: &[&'static str]) -> Result<String>
  where
    I: IntoArg,
  {
    let (res, out, _) = do_run(device, args, |c, a| args::handle_arguments(c, a));
    res.map(|_| String::from_utf8_lossy(&out).into_owned())
  }
}
