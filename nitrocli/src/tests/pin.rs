// pin.rs

// *************************************************************************
// * Copyright (C) 2019 Daniel Mueller (deso@posteo.net)                   *
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

use nitrokey::Authenticate;
use nitrokey::Device;

use super::*;

#[test_device]
fn unblock(device: nitrokey::DeviceWrapper) -> crate::Result<()> {
  let (device, err) = device.authenticate_user("wrong-pin").unwrap_err();
  assert_eq!(err, nitrokey::CommandError::WrongPassword);
  assert!(device.get_user_retry_count() < 3);

  let model = device.get_model();
  let _ = Nitrocli::with_dev(device).handle(&["pin", "unblock"])?;
  let device = nitrokey::connect_model(model)?;
  assert_eq!(device.get_user_retry_count(), 3);
  Ok(())
}

#[test_device]
fn set_user(device: nitrokey::DeviceWrapper) -> crate::Result<()> {
  let mut ncli = Nitrocli::with_dev(device);

  // Set a new user PIN.
  ncli.new_user_pin("new-pin");
  let out = ncli.handle(&["pin", "set", "user"])?;
  assert!(out.is_empty());

  let device = nitrokey::connect_model(ncli.model().unwrap())?;
  let (device, err) = device
    .authenticate_user(NITROKEY_DEFAULT_USER_PIN)
    .unwrap_err();
  assert_eq!(err, nitrokey::CommandError::WrongPassword);
  drop(device);

  // Revert to the default user PIN.
  ncli.user_pin("new-pin");
  ncli.new_user_pin(NITROKEY_DEFAULT_USER_PIN);

  let out = ncli.handle(&["pin", "set", "user"])?;
  assert!(out.is_empty());

  let device = nitrokey::connect_model(ncli.model().unwrap())?;
  let _ = device.authenticate_user(NITROKEY_DEFAULT_USER_PIN).unwrap();
  Ok(())
}
