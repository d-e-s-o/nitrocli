// otp.rs

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

use super::*;
use crate::tests::nitrocli;

#[test_device]
fn set_invalid_slot_raw(device: nitrokey::DeviceWrapper) {
  let (rc, out, err) = nitrocli::run(NO_DEV, &["otp", "set", "100", "name", "1234"]);

  assert_ne!(rc, 0);
  assert_eq!(out, b"");
  assert_eq!(&err[..24], b"Could not write OTP slot");
}

#[test_device]
fn set_invalid_slot(device: nitrokey::DeviceWrapper) {
  let res = nitrocli::handle(Some(device), &["otp", "set", "100", "name", "1234"]);
  assert_eq!(
    res.unwrap_cmd_err(),
    (
      Some("Could not write OTP slot"),
      nitrokey::CommandError::InvalidSlot
    )
  );
}
