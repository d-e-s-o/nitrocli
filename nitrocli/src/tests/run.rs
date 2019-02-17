// run.rs

// *************************************************************************
// * Copyright (C) 2019 Daniel Mueller (deso@posteo.net)                   *
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

use super::*;

#[test]
fn no_command_or_option() {
  let (rc, out, err) = Nitrocli::new().run(&[]);

  assert_ne!(rc, 0);
  assert_eq!(out, b"");

  let s = String::from_utf8_lossy(&err).into_owned();
  assert!(s.starts_with("Usage:\n"), s);
}

#[test]
fn help_option() {
  fn test(opt: &'static str) {
    let (rc, out, err) = Nitrocli::new().run(&[opt]);

    assert_eq!(rc, 0);
    assert_eq!(err, b"");

    let s = String::from_utf8_lossy(&out).into_owned();
    assert!(s.starts_with("Usage:\n"), s);
  }

  test("--help");
  test("-h")
}

#[test]
fn version_option() {
  fn test(re: &regex::Regex, opt: &'static str) {
    let (rc, out, err) = Nitrocli::new().run(&[opt]);

    assert_eq!(rc, 0);
    assert_eq!(err, b"");

    let s = String::from_utf8_lossy(&out).into_owned();
    let _ = re;
    assert!(re.is_match(&s), out);
  }

  let re = regex::Regex::new(r"^nitrocli \d+.\d+.\d+(-[^-]+)*$").unwrap();

  test(&re, "--version");
  test(&re, "-V");
}
