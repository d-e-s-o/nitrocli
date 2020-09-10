// fill.rs

// Copyright (C) 2020 The Nitrocli Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use super::*;

// Ignore this test as it takes about one hour to execute
#[ignore]
#[test_device(storage)]
fn fill(model: nitrokey::Model) -> anyhow::Result<()> {
  let res = Nitrocli::new().model(model).handle(&["fill"]);
  assert!(res.is_ok());
  Ok(())
}
