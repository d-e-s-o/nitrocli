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
use crate::tests::nitrocli;

#[test]
fn no_command_or_option() {
  let (rc, out, err) = nitrocli::run(NO_DEV, &[]);

  assert_ne!(rc, 0);
  assert_eq!(out, b"");

  let s = String::from_utf8_lossy(&err).into_owned();
  assert!(s.starts_with("Usage:\n"), s);
}

#[test]
fn help_option() {
  fn test(opt: &'static str) {
    let (rc, out, err) = nitrocli::run(NO_DEV, &[opt]);

    assert_eq!(rc, 0);
    assert_eq!(err, b"");

    let s = String::from_utf8_lossy(&out).into_owned();
    assert!(s.starts_with("Usage:\n"), s);
  }

  test("--help");
  test("-h")
}
