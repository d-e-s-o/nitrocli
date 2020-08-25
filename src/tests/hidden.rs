// hidden.rs

// Copyright (C) 2019-2020 The Nitrocli Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use super::*;

#[test_device(storage)]
fn hidden_create_open_close(model: nitrokey::Model) -> anyhow::Result<()> {
  let mut ncli = Nitrocli::new().model(model).password("1234567");
  let out = ncli.handle(&["hidden", "create", "0", "50", "100"])?;
  assert!(out.is_empty());

  let out = ncli.handle(&["hidden", "open"])?;
  assert!(out.is_empty());

  {
    let mut manager = nitrokey::force_take()?;
    let device = manager.connect_storage()?;
    assert!(!device.get_storage_status()?.encrypted_volume.active);
    assert!(device.get_storage_status()?.hidden_volume.active);
  }

  let out = ncli.handle(&["hidden", "close"])?;
  assert!(out.is_empty());

  {
    let mut manager = nitrokey::force_take()?;
    let device = manager.connect_storage()?;
    assert!(!device.get_storage_status()?.encrypted_volume.active);
    assert!(!device.get_storage_status()?.hidden_volume.active);
  }

  Ok(())
}
