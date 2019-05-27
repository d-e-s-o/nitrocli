// storage.rs

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
fn status_open_close(device: nitrokey::Storage) -> crate::Result<()> {
  fn make_re(open: Option<bool>) -> regex::Regex {
    let encrypted = match open {
      Some(open) => {
        if open {
          "active"
        } else {
          "(read-only|inactive)"
        }
      }
      None => "(read-only|active|inactive)",
    };
    let re = format!(
      r#"
    volumes:
      unencrypted:     (read-only|active|inactive)
      encrypted:       {}
      hidden:          (read-only|active|inactive)
$"#,
      encrypted
    );
    regex::Regex::new(&re).unwrap()
  }

  let mut ncli = Nitrocli::with_dev(device);
  let out = ncli.handle(&["status"])?;
  assert!(make_re(None).is_match(&out), out);

  let _ = ncli.handle(&["storage", "open"])?;
  let out = ncli.handle(&["status"])?;
  assert!(make_re(Some(true)).is_match(&out), out);

  let _ = ncli.handle(&["storage", "close"])?;
  let out = ncli.handle(&["status"])?;
  assert!(make_re(Some(false)).is_match(&out), out);

  Ok(())
}

#[test_device]
fn encrypted_open_close(device: nitrokey::Storage) -> crate::Result<()> {
  let mut ncli = Nitrocli::with_dev(device);
  let out = ncli.handle(&["storage", "open"])?;
  assert!(out.is_empty());

  let device = nitrokey::Storage::connect()?;
  assert!(device.get_status()?.encrypted_volume.active);
  assert!(!device.get_status()?.hidden_volume.active);
  drop(device);

  let out = ncli.handle(&["storage", "close"])?;
  assert!(out.is_empty());

  let device = nitrokey::Storage::connect()?;
  assert!(!device.get_status()?.encrypted_volume.active);
  assert!(!device.get_status()?.hidden_volume.active);

  Ok(())
}

#[test_device]
fn hidden_create_open_close(device: nitrokey::Storage) -> crate::Result<()> {
  let mut ncli = Nitrocli::with_dev(device);
  let out = ncli.handle(&["storage", "hidden", "create", "0", "50", "100"])?;
  assert!(out.is_empty());

  let out = ncli.handle(&["storage", "hidden", "open"])?;
  assert!(out.is_empty());

  let device = nitrokey::Storage::connect()?;
  assert!(!device.get_status()?.encrypted_volume.active);
  assert!(device.get_status()?.hidden_volume.active);
  drop(device);

  let out = ncli.handle(&["storage", "hidden", "close"])?;
  assert!(out.is_empty());

  let device = nitrokey::Storage::connect()?;
  assert!(!device.get_status()?.encrypted_volume.active);
  assert!(!device.get_status()?.hidden_volume.active);

  Ok(())
}
