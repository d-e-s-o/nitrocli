// args.rs

// *************************************************************************
// * Copyright (C) 2018-2020 Daniel Mueller (deso@posteo.net)              *
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

use std::ffi;
use std::io;
use std::result;

use crate::arg_defs;
use crate::error::Error;
use crate::RunCtx;

type Result<T> = result::Result<T, Error>;

trait Stdio {
  fn stdio(&mut self) -> (&mut dyn io::Write, &mut dyn io::Write);
}

impl<'io> Stdio for RunCtx<'io> {
  fn stdio(&mut self) -> (&mut dyn io::Write, &mut dyn io::Write) {
    (self.stdout, self.stderr)
  }
}

impl<W> Stdio for (&mut W, &mut W)
where
  W: io::Write,
{
  fn stdio(&mut self) -> (&mut dyn io::Write, &mut dyn io::Write) {
    (self.0, self.1)
  }
}

/// A command execution context that captures additional data pertaining
/// the command execution.
pub struct ExecCtx<'io> {
  pub model: Option<arg_defs::DeviceModel>,
  pub stdout: &'io mut dyn io::Write,
  pub stderr: &'io mut dyn io::Write,
  pub admin_pin: Option<ffi::OsString>,
  pub user_pin: Option<ffi::OsString>,
  pub new_admin_pin: Option<ffi::OsString>,
  pub new_user_pin: Option<ffi::OsString>,
  pub password: Option<ffi::OsString>,
  pub no_cache: bool,
  pub verbosity: u64,
}

impl<'io> Stdio for ExecCtx<'io> {
  fn stdio(&mut self) -> (&mut dyn io::Write, &mut dyn io::Write) {
    (self.stdout, self.stderr)
  }
}

/// Parse the command-line arguments and execute the selected command.
pub(crate) fn handle_arguments(ctx: &mut RunCtx<'_>, args: Vec<String>) -> Result<()> {
  use structopt::StructOpt;

  match arg_defs::Args::from_iter_safe(args.iter()) {
    Ok(args) => {
      let mut ctx = ExecCtx {
        model: args.model,
        stdout: ctx.stdout,
        stderr: ctx.stderr,
        admin_pin: ctx.admin_pin.take(),
        user_pin: ctx.user_pin.take(),
        new_admin_pin: ctx.new_admin_pin.take(),
        new_user_pin: ctx.new_user_pin.take(),
        password: ctx.password.take(),
        no_cache: ctx.no_cache,
        verbosity: args.verbose.into(),
      };
      args.cmd.execute(&mut ctx)
    }
    Err(err) => {
      if err.use_stderr() {
        Err(err.into())
      } else {
        println!(ctx, "{}", err.message)?;
        Ok(())
      }
    }
  }
}
