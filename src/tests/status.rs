// status.rs

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
fn not_found_raw() {
  let (rc, out, err) = Nitrocli::new().run(&["status"]);

  assert_ne!(rc, 0);
  assert_eq!(out, b"");
  let expected = r#"Nitrokey device not found

Caused by:
    Communication error: Could not connect to a Nitrokey device
"#;
  assert_eq!(err, expected.as_bytes());
}

#[test_device]
fn not_found() {
  let res = Nitrocli::new().handle(&["status"]);
  let err = res.unwrap_err().to_string();
  assert_eq!(err, "Nitrokey device not found");
}

#[test_device(pro)]
fn output_pro(model: nitrokey::Model) -> anyhow::Result<()> {
  let re = regex::Regex::new(
    r#"^Status:
  model:             Pro
  serial number:     0x[[:xdigit:]]{8}
  firmware version:  v\d+\.\d+
  user retry count:  [0-3]
  admin retry count: [0-3]
$"#,
  )
  .unwrap();

  let out = Nitrocli::with_model(model).handle(&["status"])?;
  assert!(re.is_match(&out), out);
  Ok(())
}

#[test_device(storage)]
fn output_storage(model: nitrokey::Model) -> anyhow::Result<()> {
  let re = regex::Regex::new(
    r#"^Status:
  model:             Storage
  serial number:     0x[[:xdigit:]]{8}
  firmware version:  v\d+\.\d+
  user retry count:  [0-3]
  admin retry count: [0-3]
  Storage:
    SD card ID:        0x[[:xdigit:]]{8}
    firmware:          (un)?locked
    storage keys:      (not )?created
    volumes:
      unencrypted:     (read-only|active|inactive)
      encrypted:       (read-only|active|inactive)
      hidden:          (read-only|active|inactive)
$"#,
  )
  .unwrap();

  let out = Nitrocli::with_model(model).handle(&["status"])?;
  assert!(re.is_match(&out), out);
  Ok(())
}
