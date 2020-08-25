// lock.rs

// Copyright (C) 2019-2020 The Nitrocli Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use super::*;

#[test_device(pro)]
fn lock_pro(model: nitrokey::Model) -> anyhow::Result<()> {
  // We can't really test much more here than just success of the command.
  let out = Nitrocli::new().model(model).handle(&["lock"])?;
  assert!(out.is_empty());

  Ok(())
}

#[test_device(storage)]
fn lock_storage(model: nitrokey::Model) -> anyhow::Result<()> {
  let mut ncli = Nitrocli::new().model(model);
  let _ = ncli.handle(&["encrypted", "open"])?;

  let out = ncli.handle(&["lock"])?;
  assert!(out.is_empty());

  let mut manager = nitrokey::force_take()?;
  let device = manager.connect_storage()?;
  assert!(!device.get_storage_status()?.encrypted_volume.active);

  Ok(())
}
