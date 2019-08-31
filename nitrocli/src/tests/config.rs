// config.rs

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

  let out = Nitrocli::make()
    .model(model)
    .build()
    .handle(&["config", "get"])?;

  assert!(re.is_match(&out), out);
  Ok(())
}

#[test_device]
fn set_wrong_usage(model: nitrokey::Model) {
  let res =
    Nitrocli::make()
      .model(model)
      .build()
      .handle(&["config", "set", "--numlock", "2", "-N"]);

  assert_eq!(
    res.unwrap_str_err(),
    "--numlock and --no-numlock are mutually exclusive"
  );
}

#[test_device]
fn set_get(model: nitrokey::Model) -> crate::Result<()> {
  let mut ncli = Nitrocli::make().model(model).build();
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
