// main.rs

// Copyright (C) 2017-2024 The Nitrocli Developers
// SPDX-License-Identifier: GPL-3.0-or-later

#![warn(
  bad_style,
  broken_intra_doc_links,
  dead_code,
  future_incompatible,
  illegal_floating_point_literal_pattern,
  improper_ctypes,
  late_bound_lifetime_arguments,
  missing_debug_implementations,
  missing_docs,
  no_mangle_generic_items,
  non_shorthand_field_patterns,
  nonstandard_style,
  overflowing_literals,
  path_statements,
  patterns_in_fns_without_body,
  proc_macro_derive_resolution_fallback,
  renamed_and_removed_lints,
  rust_2018_compatibility,
  rust_2018_idioms,
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
mod tty;

use std::env;
use std::error;
use std::ffi;
use std::fmt;
use std::io;
use std::process;
use std::str;

use structopt::clap::ErrorKind;
use structopt::clap::SubCommand;
use structopt::StructOpt;

const NITROCLI_BINARY: &str = "NITROCLI_BINARY";
const NITROCLI_RESOLVED_USB_PATH: &str = "NITROCLI_RESOLVED_USB_PATH";
const NITROCLI_MODEL: &str = "NITROCLI_MODEL";
const NITROCLI_USB_PATH: &str = "NITROCLI_USB_PATH";
const NITROCLI_VERBOSITY: &str = "NITROCLI_VERBOSITY";
const NITROCLI_NO_CACHE: &str = "NITROCLI_NO_CACHE";
const NITROCLI_SERIAL_NUMBERS: &str = "NITROCLI_SERIAL_NUMBERS";

const NITROCLI_ADMIN_PIN: &str = "NITROCLI_ADMIN_PIN";
const NITROCLI_USER_PIN: &str = "NITROCLI_USER_PIN";
const NITROCLI_NEW_ADMIN_PIN: &str = "NITROCLI_NEW_ADMIN_PIN";
const NITROCLI_NEW_USER_PIN: &str = "NITROCLI_NEW_USER_PIN";
const NITROCLI_PASSWORD: &str = "NITROCLI_PASSWORD";

/// A special error type that indicates the desire to exit directly,
/// without additional error reporting.
///
/// This error is mostly used by the extension support code so that we
/// are able to mirror the extension's exit code while preserving our
/// context logic and the fairly isolated testing it enables.
struct DirectExitError(i32);

impl fmt::Debug for DirectExitError {
  fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
    unreachable!()
  }
}

impl fmt::Display for DirectExitError {
  fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
    unreachable!()
  }
}

impl error::Error for DirectExitError {}

/// Parse the command-line arguments and execute the selected command.
fn handle_arguments(ctx: &mut Context<'_>, argv: Vec<String>) -> anyhow::Result<()> {
  let version = get_version_string();
  let clap = args::Args::clap().version(version.as_str());
  match clap.get_matches_from_safe(argv.iter()) {
    Ok(matches) => {
      let args = args::Args::from_clap(&matches);
      ctx.config.update(&args);
      args.cmd.execute(ctx)
    }
    Err(mut err) => {
      if err.kind == ErrorKind::HelpDisplayed {
        // For the convenience of the user we'd like to list the
        // available extensions in the help text. At the same time, we
        // don't want to unconditionally iterate through PATH (which may
        // contain directories with loads of files that need scanning)
        // for every command invoked. So we do that listing only if a
        // help text is actually displayed.
        let path = ctx.path.clone().unwrap_or_default();
        if let Ok(extensions) = commands::discover_extensions(&path) {
          let mut clap = args::Args::clap();
          for name in extensions {
            // Because of clap's brain dead API, we see no other way
            // but to leak the string we created here. That's okay,
            // though, because we exit in a moment anyway.
            let about = Box::leak(format!("Run the {} extension", name).into_boxed_str());
            clap = clap.subcommand(
              SubCommand::with_name(&name)
                // Use some magic number here that causes all
                // extensions to be listed after all other
                // subcommands.
                .display_order(1000)
                .about(about as &'static str),
            );
          }
          // At this point we are *pretty* sure that repeated invocation
          // will result in another error. So should be fine to unwrap
          // here.
          err = clap.get_matches_from_safe(argv.iter()).unwrap_err();
        }
      }

      if err.use_stderr() {
        Err(err.into())
      } else {
        println!(ctx, "{}", err.message)?;
        Ok(())
      }
    }
  }
}

fn get_version_string() -> String {
  let version = env!("CARGO_PKG_VERSION");
  let built_from = if let Some(git_revision) = option_env!("NITROCLI_GIT_REVISION") {
    format!(" (built from {})", git_revision)
  } else {
    "".to_string()
  };
  let libnitrokey = if let Ok(library_version) = nitrokey::get_library_version() {
    format!("libnitrokey {}", library_version)
  } else {
    "an undetectable libnitrokey version".to_string()
  };

  format!("{}{} using {}", version, built_from, libnitrokey)
}

/// The context used when running the program.
#[allow(missing_debug_implementations)]
pub struct Context<'io> {
  /// The `Read` object used as standard input throughout the program.
  pub stdin: &'io mut dyn io::Read,
  /// The `Write` object used as standard output throughout the program.
  pub stdout: &'io mut dyn io::Write,
  /// The `Write` object used as standard error throughout the program.
  pub stderr: &'io mut dyn io::Write,
  /// Whether `stdout` is a TTY.
  pub is_tty: bool,
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
  /// The configuration, usually read from configuration files and environment
  /// variables.
  pub config: config::Config,
}

impl<'io> Context<'io> {
  fn from_env<I, O, E>(
    stdin: &'io mut I,
    stdout: &'io mut O,
    stderr: &'io mut E,
    is_tty: bool,
    config: config::Config,
  ) -> Context<'io>
  where
    I: io::Read,
    O: io::Write,
    E: io::Write,
  {
    Context {
      stdin,
      stdout,
      stderr,
      is_tty,
      // The std::env module has several references to the PATH
      // environment variable, indicating that this name is considered
      // platform independent from their perspective. We do the same.
      path: env::var_os("PATH"),
      admin_pin: env::var_os(NITROCLI_ADMIN_PIN),
      user_pin: env::var_os(NITROCLI_USER_PIN),
      new_admin_pin: env::var_os(NITROCLI_NEW_ADMIN_PIN),
      new_user_pin: env::var_os(NITROCLI_NEW_USER_PIN),
      password: env::var_os(NITROCLI_PASSWORD),
      config,
    }
  }
}

fn evaluate_err(err: anyhow::Error, stderr: &mut dyn io::Write) -> i32 {
  if let Some(err) = err.root_cause().downcast_ref::<DirectExitError>() {
    err.0
  } else {
    let _ = writeln!(stderr, "{:#}", err);
    1
  }
}

fn run<'ctx, 'io: 'ctx>(ctx: &'ctx mut Context<'io>, args: Vec<String>) -> i32 {
  handle_arguments(ctx, args)
    .map(|()| 0)
    .unwrap_or_else(|err| evaluate_err(err, ctx.stderr))
}

fn main() {
  use std::io::Write;

  let mut stdin = io::stdin();
  let mut stdout = io::stdout();
  let mut stderr = io::stderr();

  let rc = match config::Config::load() {
    Ok(config) => {
      let is_tty = termion::is_tty(&stdout);
      let args = env::args().collect::<Vec<_>>();
      let ctx = &mut Context::from_env(&mut stdin, &mut stdout, &mut stderr, is_tty, config);

      run(ctx, args)
    }
    Err(err) => evaluate_err(err, &mut stderr),
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
