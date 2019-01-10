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

mod otp;
mod run;
mod status;

/// A trait simplifying checking for expected errors.
pub trait UnwrapError {
  /// Unwrap an Error::Error variant.
  fn unwrap_str_err(self) -> String;
  /// Unwrap a Error::CommandError variant.
  fn unwrap_cmd_err(self) -> (Option<&'static str>, nitrokey::CommandError);
}

impl<T> UnwrapError for crate::Result<T>
where
  T: fmt::Debug,
{
  fn unwrap_str_err(self) -> String {
    match self.unwrap_err() {
      crate::Error::Error(err) => err,
      err => panic!("Unexpected error variant found: {:?}", err),
    }
  }

  fn unwrap_cmd_err(self) -> (Option<&'static str>, nitrokey::CommandError) {
    match self.unwrap_err() {
      crate::Error::CommandError(ctx, err) => (ctx, err),
      err => panic!("Unexpected error variant found: {:?}", err),
    }
  }
}

struct Nitrocli {
  model: Option<nitrokey::Model>,
}

impl Nitrocli {
  pub fn new() -> Self {
    Self { model: None }
  }

  pub fn with_dev<D>(device: D) -> Self
  where
    D: nitrokey::Device,
  {
    let result = Self {
      model: Some(device.get_model()),
    };

    drop(device);
    result
  }

  fn model_to_arg(model: nitrokey::Model) -> &'static str {
    match model {
      nitrokey::Model::Pro => "--model=pro",
      nitrokey::Model::Storage => "--model=storage",
    }
  }

  fn do_run<F, R>(&mut self, args: &[&'static str], f: F) -> (R, Vec<u8>, Vec<u8>)
  where
    F: FnOnce(&mut crate::RunCtx<'_>, Vec<String>) -> R,
  {
    let args = ["nitrocli"]
      .into_iter()
      .cloned()
      .chain(self.model.map(Self::model_to_arg))
      .chain(args.into_iter().cloned())
      .map(ToOwned::to_owned)
      .collect();

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();

    let ctx = &mut crate::RunCtx {
      stdout: &mut stdout,
      stderr: &mut stderr,
    };

    (f(ctx, args), stdout, stderr)
  }

  /// Run `nitrocli`'s `run` function.
  pub fn run(&mut self, args: &[&'static str]) -> (i32, Vec<u8>, Vec<u8>) {
    self.do_run(args, |c, a| crate::run(c, a))
  }

  /// Run `nitrocli`'s `handle_arguments` function.
  pub fn handle(&mut self, args: &[&'static str]) -> crate::Result<String> {
    let (res, out, _) = self.do_run(args, |c, a| crate::args::handle_arguments(c, a));
    res.map(|_| String::from_utf8_lossy(&out).into_owned())
  }
}
