// main.rs

// Copyright (C) 2017-2020 The Nitrocli Developers
// SPDX-License-Identifier: GPL-3.0-or-later

#![warn(
  bad_style,
  dead_code,
  future_incompatible,
  illegal_floating_point_literal_pattern,
  improper_ctypes,
  intra_doc_link_resolution_failure,
  late_bound_lifetime_arguments,
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
mod config;
mod output;
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

/// Parse the command-line arguments and execute the selected command.
fn handle_arguments(ctx: &mut Context<'_>, args: Vec<String>) -> anyhow::Result<()> {
  use structopt::StructOpt;

  match args::Args::from_iter_safe(args.iter()) {
    Ok(args) => {
      ctx.config.update(&args);
      args.cmd.execute(ctx)
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
#[allow(missing_debug_implementations)]
pub struct Context<'io> {
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
  /// The configuration, usually read from configuration files and environment
  /// variables.
  pub config: config::Config,
}

impl<'io> Context<'io> {
  fn from_env<O, E>(stdout: &'io mut O, stderr: &'io mut E, config: config::Config) -> Context<'io>
  where
    O: io::Write,
    E: io::Write,
  {
    Context {
      stdout,
      stderr,
      admin_pin: env::var_os(NITROCLI_ADMIN_PIN),
      user_pin: env::var_os(NITROCLI_USER_PIN),
      new_admin_pin: env::var_os(NITROCLI_NEW_ADMIN_PIN),
      new_user_pin: env::var_os(NITROCLI_NEW_USER_PIN),
      password: env::var_os(NITROCLI_PASSWORD),
      config,
    }
  }
}

fn run<'ctx, 'io: 'ctx>(ctx: &'ctx mut Context<'io>, args: Vec<String>) -> i32 {
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

  let rc = match config::Config::load() {
    Ok(config) => {
      let args = env::args().collect::<Vec<_>>();
      let ctx = &mut Context::from_env(&mut stdout, &mut stderr, config);

      run(ctx, args)
    }
    Err(err) => {
      let _ = writeln!(stderr, "{:?}", err);
      1
    }
  };

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
