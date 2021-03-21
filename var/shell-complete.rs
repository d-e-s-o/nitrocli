// shell-complete.rs

// Copyright (C) 2020-2021 The Nitrocli Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use std::io;

use structopt::clap;
use structopt::StructOpt as _;

#[allow(unused)]
mod nitrocli {
  include!("../src/arg_util.rs");

  // We only need a stripped down version of the `Command` macro.
  macro_rules! Command {
    ( $(#[$docs:meta])* $name:ident, [
      $( $(#[$doc:meta])* $var:ident$(($inner:ty))? => $exec:expr, ) *
    ] ) => {
      $(#[$docs])*
      #[derive(Debug, PartialEq, structopt::StructOpt)]
      pub enum $name {
        $(
          $(#[$doc])*
          $var$(($inner))?,
        )*
      }
    };
  }

  include!("../src/args.rs");
}

/// Generate a shell completion script for nitrocli.
///
/// The script will be emitted to standard output.
#[derive(Debug, structopt::StructOpt)]
struct Args {
  /// The shell for which to generate a completion script for.
  #[structopt(possible_values = &clap::Shell::variants())]
  shell: clap::Shell,
  /// The command for which to generate the shell completion script.
  #[structopt(default_value = "nitrocli")]
  command: String,
}

fn generate_for_shell<W>(command: &str, shell: clap::Shell, output: &mut W)
where
  W: io::Write,
{
  let mut app = nitrocli::Args::clap();
  app.gen_completions_to(command, shell, output);
}

fn main() {
  let args = Args::from_args();
  generate_for_shell(&args.command, args.shell, &mut io::stdout())
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
    generate_for_shell("nitrocli", clap::Shell::Bash, &mut buffer);

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
