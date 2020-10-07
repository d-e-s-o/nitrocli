// clipboard.rs

// Copyright (C) 2020,2022 The Nitrocli Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use std::cmp;
use std::ffi;
use std::fmt;
use std::io::Write as _;
use std::os::unix::ffi::OsStrExt as _;
use std::process;
use std::str;
use std::thread;
use std::time;

use anyhow::Context as _;
use structopt::StructOpt as _;

#[derive(Clone, Copy, Debug, PartialEq, structopt::StructOpt)]
enum Selection {
  Primary,
  Secondary,
  Clipboard,
}

impl fmt::Display for Selection {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let s = match self {
      Self::Primary => "primary",
      Self::Secondary => "secondary",
      Self::Clipboard => "clipboard",
    };
    fmt::Display::fmt(s, f)
  }
}

impl str::FromStr for Selection {
  type Err = anyhow::Error;

  fn from_str(s: &str) -> Result<Selection, Self::Err> {
    match s {
      "primary" => Ok(Self::Primary),
      "secondary" => Ok(Self::Secondary),
      "clipboard" => Ok(Self::Clipboard),
      _ => Err(anyhow::anyhow!("Unexpected selection type: {}", s)),
    }
  }
}

/// Parse a duration from a string.
fn parse_duration(s: &str) -> Result<time::Duration, anyhow::Error> {
  let durations = [
    ("ms", 1),
    ("sec", 1000),
    ("s", 1000),
    ("min", 60000),
    ("m", 60000),
  ];

  for (suffix, multiplier) in &durations {
    if let Some(base) = s.strip_suffix(suffix) {
      if let Ok(count) = base.parse::<u64>() {
        return Ok(time::Duration::from_millis(count * multiplier));
      }
    }
  }

  anyhow::bail!("invalid duration provided: {}", s)
}

fn copy(selection: Selection, content: &[u8]) -> anyhow::Result<()> {
  let mut clip = process::Command::new("xclip")
    .stdin(process::Stdio::piped())
    .stdout(process::Stdio::null())
    .stderr(process::Stdio::null())
    .args(&["-selection", &selection.to_string()])
    .spawn()
    .context("Failed to execute xclip")?;

  let stdin = clip.stdin.as_mut().unwrap();
  stdin
    .write_all(content)
    .context("Failed to write to stdin")?;

  let output = clip.wait().context("Failed to wait for xclip to finish")?;
  anyhow::ensure!(output.success(), "xclip failed");
  Ok(())
}

/// Retrieve the current clipboard contents.
fn clipboard(selection: Selection) -> anyhow::Result<Vec<u8>> {
  let output = process::Command::new("xclip")
    .args(&["-out", "-selection", &selection.to_string()])
    .output()
    .context("Failed to execute xclip")?;

  anyhow::ensure!(
    output.status.success(),
    "xclip failed: {}",
    String::from_utf8_lossy(&output.stderr)
  );
  Ok(output.stdout)
}

/// Access Nitrokey OTP slots by name
#[derive(Debug, structopt::StructOpt)]
#[structopt()]
struct Args {
  /// The "selection" to use (see xclip(1)).
  #[structopt(short, long, default_value = "clipboard")]
  selection: Selection,
  /// Revert the contents of the clipboard to the previous value after
  /// this time.
  #[structopt(short, long, parse(try_from_str = parse_duration))]
  revert_after: Option<time::Duration>,
  /// The data to copy to the clipboard.
  #[structopt(name = "data")]
  data: ffi::OsString,
}

/// Revert clipboard contents after a while.
fn revert_contents(
  delay: time::Duration,
  selection: Selection,
  expected: &[u8],
  previous: &[u8],
) -> anyhow::Result<()> {
  let pid = unsafe { libc::fork() };
  match pid.cmp(&0) {
    cmp::Ordering::Equal => {
      // We are in the child. Sleep for the provided delay and then revert
      // the clipboard contents.
      thread::sleep(delay);
      // We potentially suffer from A-B-A as well as TOCTOU problems here.
      // But who's checking...
      let content = clipboard(selection).context("Failed to save clipboard contents")?;
      if content == expected {
        copy(selection, previous).context("Failed to restore original xclip content")?;
      }
      Ok(())
    }
    cmp::Ordering::Greater => {
      // We are in the parent. There is nothing to do but to exit.
      Ok(())
    }
    cmp::Ordering::Less => {
      // TODO: Could provide errno or whatever describes the failure.
      anyhow::bail!("Failed to fork")
    }
  }
}

fn main() -> anyhow::Result<()> {
  let args = Args::from_args();

  let revert = if let Some(revert_after) = args.revert_after {
    let content = match clipboard(args.selection) {
      Ok(content) => content,
      // If the clipboard/selection is "empty" xclip reports this
      // nonsense and fails. We have no other way to detect it than
      // pattern matching on its output, but we definitely want to
      // handle this case gracefully.
      Err(err) if err.to_string().contains("target STRING not available") => Vec::new(),
      e => e.context("Failed to save clipboard contents")?,
    };
    Some((revert_after, content))
  } else {
    None
  };

  copy(args.selection, args.data.as_bytes()).context("Failed to modify clipboard contents")?;

  if let Some((revert_after, previous)) = revert {
    revert_contents(
      revert_after,
      args.selection,
      args.data.as_bytes(),
      &previous,
    )
    .context("Failed to revert clipboard contents")?;
  }
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  /// Make sure that we can parse directions as expected.
  #[test]
  fn duration_parsing() {
    assert_eq!(
      parse_duration("1ms").unwrap(),
      time::Duration::from_millis(1)
    );
    assert_eq!(
      parse_duration("500ms").unwrap(),
      time::Duration::from_millis(500)
    );
    assert_eq!(parse_duration("1s").unwrap(), time::Duration::from_secs(1));
    assert_eq!(
      parse_duration("1sec").unwrap(),
      time::Duration::from_secs(1)
    );
    assert_eq!(
      parse_duration("13s").unwrap(),
      time::Duration::from_secs(13)
    );
    assert_eq!(
      parse_duration("13sec").unwrap(),
      time::Duration::from_secs(13)
    );
    assert_eq!(
      parse_duration("1m").unwrap(),
      time::Duration::from_secs(1 * 60)
    );
    assert_eq!(
      parse_duration("1min").unwrap(),
      time::Duration::from_secs(1 * 60)
    );
    assert_eq!(
      parse_duration("13m").unwrap(),
      time::Duration::from_secs(13 * 60)
    );
    assert_eq!(
      parse_duration("13min").unwrap(),
      time::Duration::from_secs(13 * 60)
    );
  }
}
