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

fn generate_bash<W>(command: &str, output: &mut W)
where
  W: io::Write,
{
  let mut app = nitrocli::Args::clap();
  app.gen_completions_to(command, clap::Shell::Bash, output);
}

fn main() {
  let args = Args::from_args();
  generate_bash(&args.command, &mut io::stdout())
}

#[cfg(test)]
mod tests {
  use super::*;

  use std::io;
  use std::ops::Add as _;
  use std::process;

  /// Separate the given words by newlines.
  fn lines<'w, W>(mut words: W) -> String
  where
    W: Iterator<Item = &'w str>,
  {
    let first = words.next().unwrap_or("");
    words
      .fold(first.to_string(), |words, word| {
        format!("{}\n{}", words, word)
      })
      .add("\n")
  }

  /// Check if `bash` is present on the system.
  fn has_bash() -> bool {
    match process::Command::new("bash").arg("-c").arg("exit").spawn() {
      // We deliberately only indicate that bash does not exist if we
      // get a file-not-found error. We don't expect any other error but
      // should there be one things will blow up later.
      Err(ref err) if err.kind() == io::ErrorKind::NotFound => false,
      _ => true,
    }
  }

  /// Perform a bash completion of the given arguments to nitrocli.
  fn complete_bash<'w, W>(words: W) -> Vec<u8>
  where
    W: ExactSizeIterator<Item = &'w str>,
  {
    let mut buffer = Vec::new();
    generate_bash("nitrocli", &mut buffer);

    let script = String::from_utf8(buffer).unwrap();
    let command = format!(
      "
set -e;
eval '{script}';
export COMP_WORDS=({words});
export COMP_CWORD={index};
_nitrocli;
echo -n ${{COMPREPLY}}
      ",
      index = words.len(),
      words = lines(Some("nitrocli").into_iter().chain(words)),
      script = script
    );

    let output = process::Command::new("bash")
      .arg("-c")
      .arg(command)
      .output()
      .unwrap();

    output.stdout
  }

  #[test]
  fn array_lines() {
    assert_eq!(&lines(vec![].into_iter()), "\n");
    assert_eq!(&lines(vec!["first"].into_iter()), "first\n");
    assert_eq!(
      &lines(vec!["first", "second"].into_iter()),
      "first\nsecond\n"
    );
    assert_eq!(
      &lines(vec!["first", "second", "third"].into_iter()),
      "first\nsecond\nthird\n"
    );
  }

  #[test]
  fn complete_all_the_things() {
    if !has_bash() {
      return;
    }

    assert_eq!(complete_bash(vec!["stat"].into_iter()), b"status");
    assert_eq!(
      complete_bash(vec!["status", "--ver"].into_iter()),
      b"--version"
    );
    assert_eq!(complete_bash(vec!["--version"].into_iter()), b"--version");
    assert_eq!(complete_bash(vec!["--model", "s"].into_iter()), b"storage");
    assert_eq!(
      complete_bash(vec!["otp", "get", "--model", "p"].into_iter()),
      b"pro"
    );
  }
}
