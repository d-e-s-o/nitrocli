// main.rs

// *************************************************************************
// * Copyright (C) 2017-2019 Daniel Mueller (deso@posteo.net)              *
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
  plugin_as_library,
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

//! Nitrocli is a program providing a command line interface to certain
//! commands of Nitrokey Pro and Storage devices.

#[macro_use]
mod redefine;
#[macro_use]
mod arg_util;

mod args;
mod commands;
mod error;
mod pinentry;
#[cfg(test)]
mod tests;

use std::env;
use std::ffi;
use std::io;
use std::process;
use std::result;

use crate::error::Error;

type Result<T> = result::Result<T, Error>;

const NITROCLI: &str = "nitrocli";
const NITROCLI_ADMIN_PIN: &str = "NITROCLI_ADMIN_PIN";
const NITROCLI_USER_PIN: &str = "NITROCLI_USER_PIN";
const NITROCLI_NEW_ADMIN_PIN: &str = "NITROCLI_NEW_ADMIN_PIN";
const NITROCLI_NEW_USER_PIN: &str = "NITROCLI_NEW_USER_PIN";
const NITROCLI_PASSWORD: &str = "NITROCLI_PASSWORD";
const NITROCLI_NO_CACHE: &str = "NITROCLI_NO_CACHE";

/// The context used when running the program.
pub(crate) struct RunCtx<'io> {
  /// The `Write` object used as standard output throughout the program.
  pub stdout: &'io mut dyn io::Write,
  /// The `Write` object used as standard error throughout the program.
  pub stderr: &'io mut dyn io::Write,
  /// The content of the `PATH` environment variable.
  pub path: Option<ffi::OsString>,
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
  match args::handle_arguments(ctx, args) {
    Ok(()) => 0,
    Err(err) => match err {
      Error::ArgparseError(err) => match err {
        // argparse printed the help message
        0 => 0,
        // argparse printed an error message
        _ => 1,
      },
      Error::ExtensionFailed(_, rc) => {
        // We let the extension itself deal with error reporting, we
        // just mirror its exit code (if any).
        rc.unwrap_or(1)
      }
      _ => {
        let _ = eprintln!(ctx, "{}", err);
        1
      }
    },
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
    path: env::var_os("PATH"),
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
