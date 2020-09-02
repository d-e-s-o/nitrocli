// run.rs

// *************************************************************************
// * Copyright (C) 2019-2020 The Nitrocli Developers                       *
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

use std::path;

use super::*;

#[test]
fn no_command_or_option() {
  let (rc, out, err) = Nitrocli::new().run(&[]);

  assert_ne!(rc, 0);
  assert_eq!(out, b"");

  let s = String::from_utf8_lossy(&err).into_owned();
  assert!(s.starts_with("nitrocli"), s);
  assert!(s.contains("USAGE:\n"), s);
}

#[test]
fn help_options() {
  fn test_run(args: &[&str], help: &str) {
    let mut all = args.to_vec();
    all.push(help);

    let (rc, out, err) = Nitrocli::new().run(&all);

    assert_eq!(rc, 0);
    assert_eq!(err, b"");

    let s = String::from_utf8_lossy(&out).into_owned();
    let mut args = args.to_vec();
    args.insert(0, "nitrocli");
    assert!(s.starts_with(&args.join("-")), s);
    assert!(s.contains("USAGE:\n"), s);
  }

  fn test(args: &[&str]) {
    test_run(args, "--help");
    test_run(args, "-h");
  }

  test(&[]);
  test(&["config"]);
  test(&["config", "get"]);
  test(&["config", "set"]);
  test(&["encrypted"]);
  test(&["encrypted", "open"]);
  test(&["encrypted", "close"]);
  test(&["hidden"]);
  test(&["hidden", "close"]);
  test(&["hidden", "create"]);
  test(&["hidden", "open"]);
  test(&["lock"]);
  test(&["otp"]);
  test(&["otp", "clear"]);
  test(&["otp", "get"]);
  test(&["otp", "set"]);
  test(&["otp", "status"]);
  test(&["pin"]);
  test(&["pin", "clear"]);
  test(&["pin", "set"]);
  test(&["pin", "unblock"]);
  test(&["pws"]);
  test(&["pws", "clear"]);
  test(&["pws", "get"]);
  test(&["pws", "set"]);
  test(&["pws", "status"]);
  test(&["reset"]);
  test(&["status"]);
  test(&["unencrypted"]);
  test(&["unencrypted", "set"]);
}

#[test]
#[ignore]
fn version_option() {
  // clap sends the version output directly to stdout: https://github.com/clap-rs/clap/issues/1390
  // Therefore we ignore this test for the time being.

  fn test(re: &regex::Regex, opt: &'static str) {
    let (rc, out, err) = Nitrocli::new().run(&[opt]);

    assert_eq!(rc, 0);
    assert_eq!(err, b"");

    let s = String::from_utf8_lossy(&out).into_owned();
    let _ = re;
    assert!(re.is_match(&s), out);
  }

  let re = regex::Regex::new(r"^nitrocli \d+.\d+.\d+(-[^-]+)*\n$").unwrap();

  test(&re, "--version");
  test(&re, "-V");
}

#[test]
fn config_file() {
  let config =
    crate::config::read_config_file(&path::Path::new("doc/config.example.toml")).unwrap();

  assert_eq!(Some(crate::args::DeviceModel::Pro), config.model);
  assert_eq!(true, config.no_cache);
  assert_eq!(2, config.verbosity);
}
