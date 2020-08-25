// list.rs

// Copyright (C) 2020 The Nitrocli Developers
// SPDX-License-Identifier: GPL-3.0-or-later

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

  let out = Nitrocli::new().model(model).handle(&["list"])?;
  assert!(re.is_match(&out), out);
  Ok(())
}
