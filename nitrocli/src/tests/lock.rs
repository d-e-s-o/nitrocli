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

#[test_device(pro)]
fn lock_pro(model: nitrokey::Model) -> crate::Result<()> {
  // We can't really test much more here than just success of the command.
  let out = Nitrocli::make().model(model).build().handle(&["lock"])?;
  assert!(out.is_empty());

  Ok(())
}

#[test_device(storage)]
fn lock_storage(model: nitrokey::Model) -> crate::Result<()> {
  let mut ncli = Nitrocli::make().model(model).build();
  let _ = ncli.handle(&["encrypted", "open"])?;

  let out = ncli.handle(&["lock"])?;
  assert!(out.is_empty());

  let mut manager = nitrokey::force_take()?;
  let device = manager.connect_storage()?;
  assert!(!device.get_status()?.encrypted_volume.active);

  Ok(())
}
