// encrypted.rs

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

#[test_device(storage)]
fn status_open_close(model: nitrokey::Model) -> crate::Result<()> {
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

  let mut ncli = Nitrocli::make().model(model).build();
  let out = ncli.handle(&["status"])?;
  assert!(make_re(None).is_match(&out), out);

  let _ = ncli.handle(&["encrypted", "open"])?;
  let out = ncli.handle(&["status"])?;
  assert!(make_re(Some(true)).is_match(&out), out);

  let _ = ncli.handle(&["encrypted", "close"])?;
  let out = ncli.handle(&["status"])?;
  assert!(make_re(Some(false)).is_match(&out), out);

  Ok(())
}

#[test_device(pro)]
fn encrypted_open_on_pro(model: nitrokey::Model) {
  let res = Nitrocli::make()
    .model(model)
    .build()
    .handle(&["encrypted", "open"]);

  assert_eq!(
    res.unwrap_str_err(),
    "This command is only available on the Nitrokey Storage",
  );
}

#[test_device(storage)]
fn encrypted_open_close(model: nitrokey::Model) -> crate::Result<()> {
  let mut ncli = Nitrocli::make().model(model).build();
  let out = ncli.handle(&["encrypted", "open"])?;
  assert!(out.is_empty());

  {
    let mut manager = nitrokey::force_take()?;
    let device = manager.connect_storage()?;
    assert!(device.get_status()?.encrypted_volume.active);
    assert!(!device.get_status()?.hidden_volume.active);
  }

  let out = ncli.handle(&["encrypted", "close"])?;
  assert!(out.is_empty());

  {
    let mut manager = nitrokey::force_take()?;
    let device = manager.connect_storage()?;
    assert!(!device.get_status()?.encrypted_volume.active);
    assert!(!device.get_status()?.hidden_volume.active);
  }

  Ok(())
}
