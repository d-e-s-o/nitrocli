// main.rs

// *************************************************************************
// * Copyright (C) 2017-2020 Daniel Mueller (deso@posteo.net)              *
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

#![warn(
  bad_style,
  dead_code,
  future_incompatible,
  illegal_floating_point_literal_pattern,
  improper_ctypes,
  intra_doc_link_resolution_failure,
  late_bound_lifetime_arguments,
  missing_copy_implementations,
  missing_debug_implementations,
  missing_docs,
  no_mangle_generic_items,
  non_shorthand_field_patterns,
  nonstandard_style,
  overflowing_literals,
  path_statements,
  patterns_in_fns_without_body,
  private_in_public,
  proc_macro_derive_resolution_fallback,
  renamed_and_removed_lints,
  rust_2018_compatibility,
  rust_2018_idioms,
  safe_packed_borrows,
  stable_features,
  trivial_bounds,
  trivial_numeric_casts,
  type_alias_bounds,
  tyvar_behind_raw_pointer,
  unconditional_recursion,
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

//! Nitrocli is a program providing a command line interface to certain
//! commands of Nitrokey Pro and Storage devices.

#[macro_use]
mod redefine;
#[macro_use]
mod arg_util;

mod args;
mod commands;
mod pinentry;
#[cfg(test)]
mod tests;

use std::env;
use std::ffi;
use std::io;
use std::process;

const NITROCLI_ADMIN_PIN: &str = "NITROCLI_ADMIN_PIN";
const NITROCLI_USER_PIN: &str = "NITROCLI_USER_PIN";
const NITROCLI_NEW_ADMIN_PIN: &str = "NITROCLI_NEW_ADMIN_PIN";
const NITROCLI_NEW_USER_PIN: &str = "NITROCLI_NEW_USER_PIN";
const NITROCLI_PASSWORD: &str = "NITROCLI_PASSWORD";
const NITROCLI_NO_CACHE: &str = "NITROCLI_NO_CACHE";

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
#[allow(missing_debug_implementations)]
pub struct ExecCtx<'io> {
  /// The Nitrokey model to use.
  pub model: Option<args::DeviceModel>,
  /// See `RunCtx::stdout`.
  pub stdout: &'io mut dyn io::Write,
  /// See `RunCtx::stderr`.
  pub stderr: &'io mut dyn io::Write,
  /// See `RunCtx::admin_pin`.
  pub admin_pin: Option<ffi::OsString>,
  /// See `RunCtx::user_pin`.
  pub user_pin: Option<ffi::OsString>,
  /// See `RunCtx::new_admin_pin`.
  pub new_admin_pin: Option<ffi::OsString>,
  /// See `RunCtx::new_user_pin`.
  pub new_user_pin: Option<ffi::OsString>,
  /// See `RunCtx::password`.
  pub password: Option<ffi::OsString>,
  /// See `RunCtx::no_cache`.
  pub no_cache: bool,
  /// The verbosity level to use for logging.
  pub verbosity: u64,
}

impl<'io> Stdio for ExecCtx<'io> {
  fn stdio(&mut self) -> (&mut dyn io::Write, &mut dyn io::Write) {
    (self.stdout, self.stderr)
  }
}

/// Parse the command-line arguments and execute the selected command.
fn handle_arguments(ctx: &mut RunCtx<'_>, args: Vec<String>) -> anyhow::Result<()> {
  use structopt::StructOpt;

  match args::Args::from_iter_safe(args.iter()) {
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

/// The context used when running the program.
pub(crate) struct RunCtx<'io> {
  /// The `Write` object used as standard output throughout the program.
  pub stdout: &'io mut dyn io::Write,
  /// The `Write` object used as standard error throughout the program.
  pub stderr: &'io mut dyn io::Write,
  /// The admin PIN, if provided through an environment variable.
  pub admin_pin: Option<ffi::OsString>,
  /// The user PIN, if provided through an environment variable.
  pub user_pin: Option<ffi::OsString>,
  /// The new admin PIN to set, if provided through an environment variable.
  ///
  /// This variable is only used by commands that change the admin PIN.
  pub new_admin_pin: Option<ffi::OsString>,
  /// The new user PIN, if provided through an environment variable.
  ///
  /// This variable is only used by commands that change the user PIN.
  pub new_user_pin: Option<ffi::OsString>,
  /// A password used by some commands, if provided through an environment variable.
  pub password: Option<ffi::OsString>,
  /// Whether to bypass the cache for all secrets or not.
  pub no_cache: bool,
}

fn run<'ctx, 'io: 'ctx>(ctx: &'ctx mut RunCtx<'io>, args: Vec<String>) -> i32 {
  match handle_arguments(ctx, args) {
    Ok(()) => 0,
    Err(err) => {
      let _ = eprintln!(ctx, "{:?}", err);
      1
    }
  }
}

fn main() {
  use std::io::Write;

  let mut stdout = io::stdout();
  let mut stderr = io::stderr();
  let args = env::args().collect::<Vec<_>>();
  let ctx = &mut RunCtx {
    stdout: &mut stdout,
    stderr: &mut stderr,
    admin_pin: env::var_os(NITROCLI_ADMIN_PIN),
    user_pin: env::var_os(NITROCLI_USER_PIN),
    new_admin_pin: env::var_os(NITROCLI_NEW_ADMIN_PIN),
    new_user_pin: env::var_os(NITROCLI_NEW_USER_PIN),
    password: env::var_os(NITROCLI_PASSWORD),
    no_cache: env::var_os(NITROCLI_NO_CACHE).is_some(),
  };

  let rc = run(ctx, args);
  // We exit the process the hard way below. The problem is that because
  // of this, buffered IO may not be flushed. So make sure to explicitly
  // flush before exiting. Note that stderr is unbuffered, alleviating
  // the need for any flushing there.
  // Ideally we would just make `main` return an i32 and let Rust deal
  // with all of this, but the `process::Termination` functionality is
  // still unstable and we have no way to convince the caller to "just
  // exit" without printing additional information.
  let _ = stdout.flush();
  process::exit(rc);
}
