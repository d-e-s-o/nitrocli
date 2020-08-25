// unencrypted.rs

// Copyright (C) 2019-2020 The Nitrocli Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use super::*;

#[test_device(storage)]
fn unencrypted_set_read_write(model: nitrokey::Model) -> anyhow::Result<()> {
  let mut ncli = Nitrocli::new().model(model);
  let out = ncli.handle(&["unencrypted", "set", "read-write"])?;
  assert!(out.is_empty());

  {
    let mut manager = nitrokey::force_take()?;
    let device = manager.connect_storage()?;
    assert!(device.get_storage_status()?.unencrypted_volume.active);
    assert!(!device.get_storage_status()?.unencrypted_volume.read_only);
  }

  let out = ncli.handle(&["unencrypted", "set", "read-only"])?;
  assert!(out.is_empty());

  {
    let mut manager = nitrokey::force_take()?;
    let device = manager.connect_storage()?;
    assert!(device.get_storage_status()?.unencrypted_volume.active);
    assert!(device.get_storage_status()?.unencrypted_volume.read_only);
  }

  Ok(())
}
