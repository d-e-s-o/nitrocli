// config.rs

// Copyright (C) 2019-2025 The Nitrocli Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use super::*;

#[test]
fn mutually_exclusive_set_options() {
  fn test(option1: &str, option2: &str) {
    let (rc, out, err) = Nitrocli::new().run(&["config", "set", option1, option2]);

    assert_ne!(rc, 0);
    assert_eq!(out, b"", "{}", String::from_utf8_lossy(&out));

    let err = String::from_utf8(err).unwrap();
    assert!(err.contains("cannot be used with"), "{}", err);
  }

  test("-c1", "-C");
  test("-o", "-O");
  test("-s1", "-S");
}

#[test_device]
fn get(model: nitrokey::Model) -> anyhow::Result<()> {
  let re = regex::Regex::new(
    r#"^Config:
  num lock binding:         (not set|\d+)
  caps lock binding:        (not set|\d+)
  scroll lock binding:      (not set|\d+)
  require user PIN for OTP: (true|false)
$"#,
  )
  .unwrap();

  let out = Nitrocli::new().model(model).handle(&["config", "get"])?;

  assert!(re.is_match(&out), "{}", out);
  Ok(())
}

#[test_device]
fn set_wrong_usage(model: nitrokey::Model) {
  let err = Nitrocli::new()
    .model(model)
    .handle(&["config", "set", "--num-lock", "2", "-N"])
    .unwrap_err()
    .to_string();

  assert!(
    err.contains("the argument '--num-lock <NUM_LOCK>' cannot be used with '--no-num-lock'"),
    "{}",
    err,
  );
}

#[test_device]
fn set_get(model: nitrokey::Model) -> anyhow::Result<()> {
  let mut ncli = Nitrocli::new().model(model);
  let _ = ncli.handle(&["config", "set", "-s", "1", "-c", "0", "-N"])?;

  let re = regex::Regex::new(
    r#"^Config:
  num lock binding:         not set
  caps lock binding:        0
  scroll lock binding:      1
  require user PIN for OTP: (true|false)
$"#,
  )
  .unwrap();

  let out = ncli.handle(&["config", "get"])?;
  assert!(re.is_match(&out), "{}", out);
  Ok(())
}
