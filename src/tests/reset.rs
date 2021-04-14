// reset.rs

// Copyright (C) 2019-2020 The Nitrocli Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use nitrokey::Authenticate;
use nitrokey::GetPasswordSafe;

use super::*;

#[test_device]
fn reset(model: nitrokey::Model) -> anyhow::Result<()> {
  let new_admin_pin = "87654321";
  let mut ncli = Nitrocli::new().model(model).new_admin_pin(new_admin_pin);

  // Change the admin PIN.
  let _ = ncli.handle(&["pin", "set", "admin"])?;

  {
    let mut manager = nitrokey::force_take()?;
    // Check that the admin PIN has been changed.
    let device = manager.connect_model(model)?;
    let _ = device.authenticate_admin(new_admin_pin).unwrap();
  }

  // Perform factory reset
  let mut ncli = ncli.admin_pin(new_admin_pin);
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

#[test_device]
fn reset_only_aes_key(model: nitrokey::Model) -> anyhow::Result<()> {
  const NEW_USER_PIN: &str = "654321";
  const NAME: &str = "slotname";
  const LOGIN: &str = "sloglogin";
  const PASSWORD: &str = "slotpassword";

  let mut ncli = Nitrocli::new().model(model).new_user_pin(NEW_USER_PIN);

  // Change the user PIN
  let _ = ncli.handle(&["pin", "set", "user"])?;

  // Add an entry to the PWS
  {
    let mut manager = nitrokey::force_take()?;
    let mut device = manager.connect_model(model)?;
    let mut pws = device.get_password_safe(NEW_USER_PIN)?;
    pws.write_slot(0, NAME, LOGIN, PASSWORD)?;
  }

  // Build AES key
  let mut ncli = Nitrocli::new().model(model);
  let out = ncli.handle(&["reset", "--only-aes-key"])?;
  assert!(out.is_empty());

  // Check that 1) the password store works, i.e., there is an AES key, that 2) we can no longer
  // access the stored data, i. e. the AES has been replaced, and that 3) the changed admin PIN
  // still works, i. e. we did not perform a factory reset.
  {
    let mut manager = nitrokey::force_take()?;
    let mut device = manager.connect_model(model)?;
    let pws = device.get_password_safe(NEW_USER_PIN)?;
    let slot = pws.get_slot_unchecked(0)?;

    if let Ok(name) = slot.get_name() {
      assert_ne!(NAME, &name);
    }
    if let Ok(login) = slot.get_login() {
      assert_ne!(LOGIN, &login);
    }
    if let Ok(password) = slot.get_password() {
      assert_ne!(PASSWORD, &password);
    }
  }

  // Reset the admin PIN for other tests
  let mut ncli = ncli.user_pin(NEW_USER_PIN).new_user_pin(nitrokey::DEFAULT_USER_PIN);
  let out = ncli.handle(&["pin", "set", "user"])?;
  assert!(out.is_empty());

  Ok(())
}
