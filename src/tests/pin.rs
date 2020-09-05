// pin.rs

// Copyright (C) 2019-2020 The Nitrocli Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use nitrokey::Authenticate;
use nitrokey::Device;

use super::*;

#[test_device]
fn unblock(model: nitrokey::Model) -> anyhow::Result<()> {
  {
    let mut manager = nitrokey::force_take()?;
    let device = manager.connect_model(model)?;
    let (device, err) = device.authenticate_user("wrong-pin").unwrap_err();
    match err {
      nitrokey::Error::CommandError(err) if err == nitrokey::CommandError::WrongPassword => (),
      _ => panic!("Unexpected error variant found: {:?}", err),
    }
    assert!(device.get_user_retry_count()? < 3);
  }

  let _ = Nitrocli::with_model(model).handle(&["pin", "unblock"])?;

  {
    let mut manager = nitrokey::force_take()?;
    let device = manager.connect_model(model)?;
    assert_eq!(device.get_user_retry_count()?, 3);
  }
  Ok(())
}

#[test_device]
fn set_user(model: nitrokey::Model) -> anyhow::Result<()> {
  let mut ncli = Nitrocli::with_model(model);
  // Set a new user PIN.
  ncli.new_user_pin("new-pin");
  let out = ncli.handle(&["pin", "set", "user"])?;
  assert!(out.is_empty());

  {
    let mut manager = nitrokey::force_take()?;
    let device = manager.connect_model(model)?;
    let (_, err) = device
      .authenticate_user(nitrokey::DEFAULT_USER_PIN)
      .unwrap_err();

    match err {
      nitrokey::Error::CommandError(err) if err == nitrokey::CommandError::WrongPassword => (),
      _ => panic!("Unexpected error variant found: {:?}", err),
    }
  }

  // Revert to the default user PIN.
  ncli.user_pin("new-pin");
  ncli.new_user_pin(nitrokey::DEFAULT_USER_PIN);

  let out = ncli.handle(&["pin", "set", "user"])?;
  assert!(out.is_empty());

  {
    let mut manager = nitrokey::force_take()?;
    let device = manager.connect_model(model)?;
    let _ = device
      .authenticate_user(nitrokey::DEFAULT_USER_PIN)
      .unwrap();
  }
  Ok(())
}
