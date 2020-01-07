// config.rs

// *************************************************************************
// * Copyright (C) 2019-2020 Daniel Mueller (deso@posteo.net)              *
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

#[test_device]
fn get(model: nitrokey::Model) -> crate::Result<()> {
  let re = regex::Regex::new(
    r#"^Config:
  numlock binding:          (not set|\d+)
  capslock binding:         (not set|\d+)
  scrollock binding:        (not set|\d+)
  require user PIN for OTP: (true|false)
$"#,
  )
  .unwrap();

  let out = Nitrocli::with_model(model).handle(&["config", "get"])?;
  assert!(re.is_match(&out), out);
  Ok(())
}

#[test_device]
fn set_wrong_usage(model: nitrokey::Model) {
  let res = Nitrocli::with_model(model).handle(&["config", "set", "--numlock", "2", "-N"]);
  let err = res.unwrap_str_err();
  assert!(
    err.contains("The argument '--numlock <numlock>' cannot be used with '--no-numlock'"),
    err,
  );
}

#[test_device]
fn set_get(model: nitrokey::Model) -> crate::Result<()> {
  let mut ncli = Nitrocli::with_model(model);
  let _ = ncli.handle(&["config", "set", "-s", "1", "-c", "0", "-N"])?;

  let re = regex::Regex::new(
    r#"^Config:
  numlock binding:          not set
  capslock binding:         0
  scrollock binding:        1
  require user PIN for OTP: (true|false)
$"#,
  )
  .unwrap();

  let out = ncli.handle(&["config", "get"])?;
  assert!(re.is_match(&out), out);
  Ok(())
}
