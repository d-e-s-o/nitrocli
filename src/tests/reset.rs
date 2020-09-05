// reset.rs

// Copyright (C) 2019-2020 The Nitrocli Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use nitrokey::Authenticate;
use nitrokey::GetPasswordSafe;

use super::*;

#[test_device]
fn reset(model: nitrokey::Model) -> anyhow::Result<()> {
  let new_admin_pin = "87654321";
  let mut ncli = Nitrocli::with_model(model);

  // Change the admin PIN.
  ncli.new_admin_pin(new_admin_pin);
  let _ = ncli.handle(&["pin", "set", "admin"])?;

  {
    let mut manager = nitrokey::force_take()?;
    // Check that the admin PIN has been changed.
    let device = manager.connect_model(model)?;
    let _ = device.authenticate_admin(new_admin_pin).unwrap();
  }

  // Perform factory reset
  ncli.admin_pin(new_admin_pin);
  let out = ncli.handle(&["reset"])?;
  assert!(out.is_empty());

  {
    let mut manager = nitrokey::force_take()?;
    // Check that the admin PIN has been reset.
    let device = manager.connect_model(model)?;
    let mut device = device
      .authenticate_admin(nitrokey::DEFAULT_ADMIN_PIN)
      .unwrap();

    // Check that the password store works, i.e., the AES key has been
    // built.
    let _ = device.get_password_safe(nitrokey::DEFAULT_USER_PIN)?;
  }

  Ok(())
}
