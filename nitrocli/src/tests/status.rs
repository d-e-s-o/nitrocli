// status.rs

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
use crate::Result;

// This test acts as verification that conversion of Error::Error
// variants into the proper exit code works properly.
#[test_device]
fn status_not_found() -> Result<()> {
  let (rc, out, err) = nitrocli::run(NO_DEV, &["status"]);

  assert_ne!(rc, 0);
  assert_eq!(out, b"");
  assert_eq!(err, b"Nitrokey device not found\n");
  Ok(())
}

#[test_device]
fn status(device: nitrokey::DeviceWrapper) -> Result<()> {
  let re = regex::Regex::new(
    r#"^Status:
  model:             (Pro|Storage)
  serial number:     0x[[:xdigit:]]{8}
  firmware version:  \d+.\d+
  user retry count:  [0-3]
  admin retry count: [0-3]
$"#,
  )
  .unwrap();

  let out = nitrocli::handle(Some(device), &["status"])?;
  assert!(re.is_match(&out), out);
  Ok(())
}
