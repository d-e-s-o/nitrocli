// pws.rs

// Copyright (C) 2019-2020 The Nitrocli Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use super::*;

#[test_device]
fn set_invalid_slot(model: nitrokey::Model) {
  let err = Nitrocli::new()
    .model(model)
    .handle(&["pws", "set", "100", "name", "login", "1234"])
    .unwrap_err()
    .to_string();

  assert_eq!(err, "Failed to write PWS slot");
}

#[test_device]
fn status(model: nitrokey::Model) -> anyhow::Result<()> {
  let re = regex::Regex::new(
    r#"^slot\tname
(\d+\t.+\n)+$"#,
  )
  .unwrap();

  let mut ncli = Nitrocli::new().model(model);
  // Make sure that we have at least something to display by ensuring
  // that there are there is one slot programmed.
  let _ = ncli.handle(&["pws", "set", "0", "the-name", "the-login", "123456"])?;

  let out = ncli.handle(&["pws", "status"])?;
  assert!(re.is_match(&out), out);
  Ok(())
}

#[test_device]
fn set_get(model: nitrokey::Model) -> anyhow::Result<()> {
  const NAME: &str = "dropbox";
  const LOGIN: &str = "d-e-s-o";
  const PASSWORD: &str = "my-secret-password";

  let mut ncli = Nitrocli::new().model(model);
  let _ = ncli.handle(&["pws", "set", "1", &NAME, &LOGIN, &PASSWORD])?;

  let out = ncli.handle(&["pws", "get", "1", "--quiet", "--name"])?;
  assert_eq!(out, format!("{}\n", NAME));

  let out = ncli.handle(&["pws", "get", "1", "--quiet", "--login"])?;
  assert_eq!(out, format!("{}\n", LOGIN));

  let out = ncli.handle(&["pws", "get", "1", "--quiet", "--password"])?;
  assert_eq!(out, format!("{}\n", PASSWORD));

  let out = ncli.handle(&["pws", "get", "1", "--quiet"])?;
  assert_eq!(out, format!("{}\n{}\n{}\n", NAME, LOGIN, PASSWORD));

  let out = ncli.handle(&["pws", "get", "1"])?;
  assert_eq!(
    out,
    format!(
      "name:     {}\nlogin:    {}\npassword: {}\n",
      NAME, LOGIN, PASSWORD
    ),
  );
  Ok(())
}

#[test_device]
fn set_reset_get(model: nitrokey::Model) -> anyhow::Result<()> {
  const NAME: &str = "some/svc";
  const LOGIN: &str = "a\\user";
  const PASSWORD: &str = "!@&-)*(&+%^@";

  let mut ncli = Nitrocli::new().model(model);
  let _ = ncli.handle(&["pws", "set", "2", &NAME, &LOGIN, &PASSWORD])?;

  let out = ncli.handle(&["reset"])?;
  assert_eq!(out, "");

  let res = ncli.handle(&["pws", "get", "2"]);
  let err = res.unwrap_err().to_string();
  assert_eq!(err, "Failed to access PWS slot");
  Ok(())
}

#[test_device]
fn clear(model: nitrokey::Model) -> anyhow::Result<()> {
  let mut ncli = Nitrocli::new().model(model);
  let _ = ncli.handle(&["pws", "set", "10", "clear-test", "some-login", "abcdef"])?;
  let _ = ncli.handle(&["pws", "clear", "10"])?;
  let res = ncli.handle(&["pws", "get", "10"]);

  let err = res.unwrap_err().to_string();
  assert_eq!(err, "Failed to access PWS slot");
  Ok(())
}
