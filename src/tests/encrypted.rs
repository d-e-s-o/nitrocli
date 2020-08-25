// encrypted.rs

// Copyright (C) 2019-2020 The Nitrocli Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use super::*;

#[test_device(storage)]
fn status_open_close(model: nitrokey::Model) -> anyhow::Result<()> {
  fn make_re(open: Option<bool>) -> regex::Regex {
    let encrypted = match open {
      Some(open) => {
        if open {
          "active"
        } else {
          "(read-only|inactive)"
        }
      }
      None => "(read-only|active|inactive)",
    };
    let re = format!(
      r#"
    volumes:
      unencrypted:     (read-only|active|inactive)
      encrypted:       {}
      hidden:          (read-only|active|inactive)
$"#,
      encrypted
    );
    regex::Regex::new(&re).unwrap()
  }

  let mut ncli = Nitrocli::new().model(model);
  let out = ncli.handle(&["status"])?;
  assert!(make_re(None).is_match(&out), out);

  let _ = ncli.handle(&["encrypted", "open"])?;
  let out = ncli.handle(&["status"])?;
  assert!(make_re(Some(true)).is_match(&out), out);

  let _ = ncli.handle(&["encrypted", "close"])?;
  let out = ncli.handle(&["status"])?;
  assert!(make_re(Some(false)).is_match(&out), out);

  Ok(())
}

#[test_device(pro)]
fn encrypted_open_on_pro(model: nitrokey::Model) {
  let err = Nitrocli::new()
    .model(model)
    .handle(&["encrypted", "open"])
    .unwrap_err()
    .to_string();

  assert_eq!(
    err,
    "This command is only available on the Nitrokey Storage",
  );
}

#[test_device(storage)]
fn encrypted_open_close(model: nitrokey::Model) -> anyhow::Result<()> {
  let mut ncli = Nitrocli::new().model(model);
  let out = ncli.handle(&["encrypted", "open"])?;
  assert!(out.is_empty());

  {
    let mut manager = nitrokey::force_take()?;
    let device = manager.connect_storage()?;
    assert!(device.get_storage_status()?.encrypted_volume.active);
    assert!(!device.get_storage_status()?.hidden_volume.active);
  }

  let out = ncli.handle(&["encrypted", "close"])?;
  assert!(out.is_empty());

  {
    let mut manager = nitrokey::force_take()?;
    let device = manager.connect_storage()?;
    assert!(!device.get_storage_status()?.encrypted_volume.active);
    assert!(!device.get_storage_status()?.hidden_volume.active);
  }

  Ok(())
}
