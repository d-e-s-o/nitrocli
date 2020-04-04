// shell-complete.rs

// *************************************************************************
// * Copyright (C) 2020 Daniel Mueller (deso@posteo.net)                   *
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

use std::io;

use structopt::clap;
use structopt::StructOpt as _;

#[allow(unused)]
mod nitrocli {
  include!("../src/arg_util.rs");

  // We only need a stripped down version of the `Command` macro.
  macro_rules! Command {
    ( $name:ident, [ $( $(#[$doc:meta])* $var:ident$(($inner:ty))? => $exec:expr, ) *] ) => {
      #[derive(Debug, PartialEq, structopt::StructOpt)]
      pub enum $name {
        $(
          $(#[$doc])*
          $var$(($inner))?,
        )*
      }
    };
  }

  include!("../src/arg_defs.rs");
}

/// Generate a bash completion script for nitrocli.
///
/// The script will be emitted to standard output.
#[derive(Debug, structopt::StructOpt)]
pub struct Args {
  /// The command for which to generate the bash completion script.
  #[structopt(default_value = "nitrocli")]
  pub command: String,
}

fn main() {
  let args = Args::from_args();
  let mut app = nitrocli::Args::clap();
  app.gen_completions_to(&args.command, clap::Shell::Bash, &mut io::stdout());
}
