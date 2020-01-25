// status.rs

// Copyright (C) 2019-2020 The Nitrocli Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use super::*;

#[test_device]
fn not_found_raw() {
  let (rc, out, err) = Nitrocli::new().run(&["status"]);

  assert_ne!(rc, 0);
  assert_eq!(out, b"");
  assert_eq!(err, b"Nitrokey device not found\n");
}

#[test_device]
fn not_found() {
  let res = Nitrocli::new().handle(&["status"]);
  let err = res.unwrap_err().to_string();
  assert_eq!(err, "Nitrokey device not found");
}

#[test_device]
fn not_found_pro() {
  let res = Nitrocli::new().handle(&["status", "--model=pro"]);
  let err = res.unwrap_err().to_string();
  assert_eq!(err, "Nitrokey device not found (filter: model=pro)");
}

#[test_device]
fn not_found_by_serial_number() {
  let res = Nitrocli::new().handle(&["status", "--model=storage", "--serial-number=deadbeef"]);
  let err = res.unwrap_err().to_string();
  assert_eq!(
    err,
    "Nitrokey device not found (filter: model=storage, serial number in [0xdeadbeef])"
  );
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

  let out = Nitrocli::new().model(model).handle(&["status"])?;
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

  let out = Nitrocli::new().model(model).handle(&["status"])?;
  assert!(re.is_match(&out), out);
  Ok(())
}
