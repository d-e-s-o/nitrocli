// status.rs

// Copyright (C) 2019-2021 The Nitrocli Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use super::*;

#[test_device]
fn not_found_raw() {
  let (rc, out, err) = Nitrocli::new().run(&["status"]);

  assert_ne!(rc, 0);
  assert_eq!(out, b"", "{}", String::from_utf8_lossy(&out));
  assert_eq!(
    err,
    b"Nitrokey device not found\n",
    "{}",
    String::from_utf8_lossy(&err)
  );
}

#[test_device]
fn not_found() {
  let res = Nitrocli::new().handle(&["status"]);
  let err = res.unwrap_err().to_string();
  assert_eq!(err, "Nitrokey device not found");
}

#[test_device(librem)]
fn output_librem(model: nitrokey::Model) -> anyhow::Result<()> {
  let re = regex::Regex::new(
    r#"^Status:
  model:             Librem Key
  serial number:     0x[[:xdigit:]]{8}
  firmware version:  v\d+\.\d+
  user retry count:  [0-3]
  admin retry count: [0-3]
$"#,
  )
  .unwrap();

  let out = Nitrocli::new().model(model).handle(&["status"])?;
  assert!(re.is_match(&out), "{}", out);
  Ok(())
}

#[test_device(pro)]
fn output_pro(model: nitrokey::Model) -> anyhow::Result<()> {
  let re = regex::Regex::new(
    r#"^Status:
  model:             Nitrokey Pro
  serial number:     0x[[:xdigit:]]{8}
  firmware version:  v\d+\.\d+
  user retry count:  [0-3]
  admin retry count: [0-3]
$"#,
  )
  .unwrap();

  let out = Nitrocli::new().model(model).handle(&["status"])?;
  assert!(re.is_match(&out), "{}", out);
  Ok(())
}

#[test_device(storage)]
fn output_storage(model: nitrokey::Model) -> anyhow::Result<()> {
  let re = regex::Regex::new(
    r#"^Status:
  model:             Nitrokey Storage
  serial number:     0x[[:xdigit:]]{8}
  firmware version:  v\d+\.\d+
  user retry count:  [0-3]
  admin retry count: [0-3]
  Storage:
    SD card ID:        0x[[:xdigit:]]{8}
    SD card usage:     \d+% .. \d+% not written
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
  assert!(re.is_match(&out), "{}", out);
  Ok(())
}
