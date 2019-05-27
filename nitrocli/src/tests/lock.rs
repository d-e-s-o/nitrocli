// lock.rs

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

#[test_device]
fn lock_pro(device: nitrokey::Pro) -> crate::Result<()> {
  // We can't really test much more here than just success of the command.
  let out = Nitrocli::with_dev(device).handle(&["lock"])?;
  assert!(out.is_empty());

  Ok(())
}

#[test_device]
fn lock_storage(device: nitrokey::Storage) -> crate::Result<()> {
  let mut ncli = Nitrocli::with_dev(device);
  let _ = ncli.handle(&["encrypted", "open"])?;

  let out = ncli.handle(&["lock"])?;
  assert!(out.is_empty());

  let device = nitrokey::Storage::connect()?;
  assert!(!device.get_status()?.encrypted_volume.active);

  Ok(())
}
