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

mod commands;
mod error;
mod pinentry;

use std::process;
use std::result;

use nitrokey;

use crate::error::Error;

type Result<T> = result::Result<T, Error>;

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

  commands!(&argv[1], [commands::status, commands::open, commands::close, commands::clear]);
}

fn main() {
  process::exit(run());
}
