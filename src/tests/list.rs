// list.rs

// *************************************************************************
// * Copyright (C) 2020 The Nitrocli Developers                            *
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
fn not_connected() -> anyhow::Result<()> {
  let res = Nitrocli::new().handle(&["list"])?;
  assert_eq!(res, "No Nitrokey device connected\n");

  Ok(())
}

#[test_device]
fn connected(model: nitrokey::Model) -> anyhow::Result<()> {
  let re = regex::Regex::new(
    r#"^device path\tmodel\tserial number
([[:^space:]]+\t(Pro|Storage|unknown)\t0x[[:xdigit:]]+
)+$"#,
  )
  .unwrap();

  let out = Nitrocli::with_model(model).handle(&["list"])?;
  assert!(re.is_match(&out), out);
  Ok(())
}
