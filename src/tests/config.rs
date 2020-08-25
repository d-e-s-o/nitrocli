// config.rs

// Copyright (C) 2019-2020 The Nitrocli Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use super::*;

#[test]
fn mutually_exclusive_set_options() {
  fn test(option1: &str, option2: &str) {
    let (rc, out, err) = Nitrocli::new().run(&["config", "set", option1, option2]);

    assert_ne!(rc, 0);
    assert_eq!(out, b"");

    let err = String::from_utf8(err).unwrap();
    assert!(err.contains("cannot be used with"), err);
  }

  test("-c", "-C");
  test("-o", "-O");
  test("-s", "-S");
}

#[test_device]
fn get(model: nitrokey::Model) -> anyhow::Result<()> {
  let re = regex::Regex::new(
    r#"^Config:
  numlock binding:          (not set|\d+)
  capslock binding:         (not set|\d+)
  scrollock binding:        (not set|\d+)
  require user PIN for OTP: (true|false)
$"#,
  )
  .unwrap();

  let out = Nitrocli::new().model(model).handle(&["config", "get"])?;

  assert!(re.is_match(&out), out);
  Ok(())
}

#[test_device]
fn set_wrong_usage(model: nitrokey::Model) {
  let err = Nitrocli::new()
    .model(model)
    .handle(&["config", "set", "--numlock", "2", "-N"])
    .unwrap_err()
    .to_string();

  assert!(
    err.contains("The argument '--numlock <numlock>' cannot be used with '--no-numlock'"),
    err,
  );
}

#[test_device]
fn set_get(model: nitrokey::Model) -> anyhow::Result<()> {
  let mut ncli = Nitrocli::new().model(model);
  let _ = ncli.handle(&["config", "set", "-s", "1", "-c", "0", "-N"])?;

  let re = regex::Regex::new(
    r#"^Config:
  numlock binding:          not set
  capslock binding:         0
  scrollock binding:        1
  require user PIN for OTP: (true|false)
$"#,
  )
  .unwrap();

  let out = ncli.handle(&["config", "get"])?;
  assert!(re.is_match(&out), out);
  Ok(())
}
