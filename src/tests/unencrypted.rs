// unencrypted.rs

// *************************************************************************
// * Copyright (C) 2019-2020 The Nitrocli Developers                       *
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

#[test_device(storage)]
fn unencrypted_set_read_write(model: nitrokey::Model) -> anyhow::Result<()> {
  let mut ncli = Nitrocli::with_model(model);
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
