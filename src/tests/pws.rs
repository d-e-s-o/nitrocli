// pws.rs

// Copyright (C) 2019-2021 The Nitrocli Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use super::*;

fn clear_pws(model: nitrokey::Model) -> anyhow::Result<()> {
  use nitrokey::GetPasswordSafe as _;

  let mut manager = nitrokey::force_take()?;
  let mut device = manager.connect_model(model)?;
  let mut pws = device.get_password_safe(nitrokey::DEFAULT_USER_PIN)?;
  let slots_to_clear: Vec<_> = pws
    .get_slots()?
    .into_iter()
    .flatten()
    .map(|s| s.index())
    .collect();
  for slot in slots_to_clear {
    pws.erase_slot(slot)?;
  }
  Ok(())
}

fn assert_slot(
  model: nitrokey::Model,
  slot: u8,
  name: &str,
  login: &str,
  password: &str,
) -> anyhow::Result<()> {
  let mut ncli = Nitrocli::new().model(model);
  let out = ncli.handle(&["pws", "get", &slot.to_string(), "--quiet"])?;
  assert_eq!(format!("{}\n{}\n{}\n", name, login, password), out);
  Ok(())
}

#[test_device]
fn add_invalid_slot(model: nitrokey::Model) {
  let err = Nitrocli::new()
    .model(model)
    .handle(&["pws", "add", "--slot", "100", "name", "login", "1234"])
    .unwrap_err()
    .to_string();

  assert_eq!(err, "Encountered invalid slot index: 100");
}

#[test_device]
fn status(model: nitrokey::Model) -> anyhow::Result<()> {
  let re = regex::Regex::new(
    r#"^slot\tname
(\d+\t.+\n)+$"#,
  )
  .unwrap();

  clear_pws(model)?;

  let mut ncli = Nitrocli::new().model(model);
  // Make sure that we have at least something to display by ensuring
  // that there are there is one slot programmed.
  let _ = ncli.handle(&["pws", "add", "the-name", "the-login", "123456"])?;

  let out = ncli.handle(&["pws", "status"])?;
  assert!(re.is_match(&out), "{}", out);
  Ok(())
}

#[test_device]
fn add_get(model: nitrokey::Model) -> anyhow::Result<()> {
  const NAME: &str = "dropbox";
  const LOGIN: &str = "d-e-s-o";
  const PASSWORD: &str = "my-secret-password";

  clear_pws(model)?;

  let mut ncli = Nitrocli::new().model(model);
  let _ = ncli.handle(&["pws", "add", "--slot", "1", &NAME, &LOGIN, &PASSWORD])?;

  let out = ncli.handle(&["pws", "get", "1", "--quiet", "--name"])?;
  assert_eq!(out, format!("{}\n", NAME));

  let out = ncli.handle(&["pws", "get", "1", "--quiet", "--login"])?;
  assert_eq!(out, format!("{}\n", LOGIN));

  let out = ncli.handle(&["pws", "get", "1", "--quiet", "--password"])?;
  assert_eq!(out, format!("{}\n", PASSWORD));

  assert_slot(model, 1, NAME, LOGIN, PASSWORD)?;

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
fn add_empty(model: nitrokey::Model) -> anyhow::Result<()> {
  let mut ncli = Nitrocli::new().model(model);

  clear_pws(model)?;

  let _ = ncli.handle(&["pws", "add", "--slot", "1", "", "", ""])?;

  let out = ncli.handle(&["pws", "get", "1", "--quiet", "--name"])?;
  assert_eq!(out, "\n");

  let out = ncli.handle(&["pws", "get", "1", "--quiet", "--login"])?;
  assert_eq!(out, "\n");

  let out = ncli.handle(&["pws", "get", "1", "--quiet", "--password"])?;
  assert_eq!(out, "\n");

  assert_slot(model, 1, "", "", "")?;

  let out = ncli.handle(&["pws", "get", "1"])?;
  assert_eq!(out, "name:     \nlogin:    \npassword: \n",);
  Ok(())
}

#[test_device]
fn add_reset_get(model: nitrokey::Model) -> anyhow::Result<()> {
  const NAME: &str = "some/svc";
  const LOGIN: &str = "a\\user";
  const PASSWORD: &str = "!@&-)*(&+%^@";

  clear_pws(model)?;

  let mut ncli = Nitrocli::new().model(model);
  let _ = ncli.handle(&["pws", "add", "--slot", "2", &NAME, &LOGIN, &PASSWORD])?;

  let out = ncli.handle(&["reset"])?;
  assert_eq!(out, "");

  let res = ncli.handle(&["pws", "get", "2"]);
  let err = res.unwrap_err().to_string();
  assert_eq!(err, "Failed to access PWS slot");
  Ok(())
}

#[test_device]
fn clear(model: nitrokey::Model) -> anyhow::Result<()> {
  clear_pws(model)?;

  let mut ncli = Nitrocli::new().model(model);
  let _ = ncli.handle(&["pws", "clear", "10"])?;
  let _ = ncli.handle(&[
    "pws",
    "add",
    "--slot",
    "10",
    "clear-test",
    "some-login",
    "abcdef",
  ])?;
  let _ = ncli.handle(&["pws", "clear", "10"])?;
  let res = ncli.handle(&["pws", "get", "10"]);

  let err = res.unwrap_err().to_string();
  assert_eq!(err, "Failed to access PWS slot");
  Ok(())
}

#[test_device]
fn update_unprogrammed(model: nitrokey::Model) -> anyhow::Result<()> {
  clear_pws(model)?;

  let mut ncli = Nitrocli::new().model(model);
  let res = ncli.handle(&["pws", "update", "10", "--name", "test"]);

  let err = res.unwrap_err().to_string();
  assert_eq!(err, "Failed to query PWS slot");
  Ok(())
}

#[test_device]
fn update_no_options(model: nitrokey::Model) -> anyhow::Result<()> {
  let mut ncli = Nitrocli::new().model(model);
  let res = ncli.handle(&["pws", "update", "10"]);

  let err = res.unwrap_err().to_string();
  assert_eq!(
    err,
    "You have to set at least one of --name, --login, or --password"
  );
  Ok(())
}

#[test_device]
fn update(model: nitrokey::Model) -> anyhow::Result<()> {
  const NAME_BEFORE: &str = "name-before";
  const NAME_AFTER: &str = "name-after";
  const LOGIN_BEFORE: &str = "login-before";
  const LOGIN_AFTER: &str = "login-after";
  const PASSWORD_BEFORE: &str = "password-before";
  const PASSWORD_AFTER: &str = "password-after";

  clear_pws(model)?;

  let mut ncli = Nitrocli::new().model(model);
  let _ = ncli.handle(&[
    "pws",
    "add",
    "--slot",
    "10",
    NAME_BEFORE,
    LOGIN_BEFORE,
    PASSWORD_BEFORE,
  ])?;

  assert_slot(model, 10, NAME_BEFORE, LOGIN_BEFORE, PASSWORD_BEFORE)?;

  let _ = ncli.handle(&["pws", "update", "10", "--name", NAME_AFTER])?;
  assert_slot(model, 10, NAME_AFTER, LOGIN_BEFORE, PASSWORD_BEFORE)?;

  let _ = ncli.handle(&["pws", "update", "10", "--login", LOGIN_AFTER])?;
  assert_slot(model, 10, NAME_AFTER, LOGIN_AFTER, PASSWORD_BEFORE)?;

  let _ = ncli.handle(&["pws", "update", "10", "--password", PASSWORD_AFTER])?;
  assert_slot(model, 10, NAME_AFTER, LOGIN_AFTER, PASSWORD_AFTER)?;

  let _ = ncli.handle(&[
    "pws",
    "update",
    "10",
    "--name",
    NAME_BEFORE,
    "--login",
    LOGIN_BEFORE,
    "--password",
    PASSWORD_BEFORE,
  ])?;
  assert_slot(model, 10, NAME_BEFORE, LOGIN_BEFORE, PASSWORD_BEFORE)?;

  Ok(())
}

#[test_device]
fn add_full(model: nitrokey::Model) -> anyhow::Result<()> {
  let mut ncli = Nitrocli::new().model(model);

  clear_pws(model)?;

  // Fill all PWS slots
  {
    use nitrokey::GetPasswordSafe as _;

    let mut manager = nitrokey::force_take()?;
    let mut device = manager.connect_model(model)?;
    let mut pws = device.get_password_safe(nitrokey::DEFAULT_USER_PIN)?;
    for slot in 0..pws.get_slot_count() {
      pws.write_slot(slot, "name", "login", "passw0rd")?;
    }
  }

  // Try to add another one
  let res = ncli.handle(&["pws", "add", "name", "login", "passw0rd"]);

  let err = res.unwrap_err().to_string();
  assert_eq!(err, "All PWS slots are already programmed");
  Ok(())
}

#[test_device]
fn add_existing(model: nitrokey::Model) -> anyhow::Result<()> {
  clear_pws(model)?;

  let mut ncli = Nitrocli::new().model(model);

  // Fill slot 0
  let _ = ncli.handle(&["pws", "add", "--slot", "0", "name0", "login0", "pass0rd"])?;

  // Try to add slot 0
  let res = ncli.handle(&["pws", "add", "--slot", "0", "name", "login", "passw0rd"]);

  let err = res.unwrap_err().to_string();
  assert_eq!(err, "The PWS slot 0 is already programmed");
  Ok(())
}

#[test_device]
fn add_slot(model: nitrokey::Model) -> anyhow::Result<()> {
  clear_pws(model)?;

  let mut ncli = Nitrocli::new().model(model);

  // Fill slots 0 and 5
  let _ = ncli.handle(&["pws", "add", "--slot", "0", "name0", "login0", "passw0rd"])?;
  let _ = ncli.handle(&["pws", "add", "--slot", "5", "name5", "login5", "passw5rd"])?;

  // Try to add slot 1
  let out = ncli.handle(&["pws", "add", "--slot", "1", "name1", "login1", "passw1rd"])?;
  assert_eq!("Added PWS slot 1\n", out);

  assert_slot(model, 0, "name0", "login0", "passw0rd")?;
  assert_slot(model, 1, "name1", "login1", "passw1rd")?;
  assert_slot(model, 5, "name5", "login5", "passw5rd")?;

  Ok(())
}

#[test_device]
fn add(model: nitrokey::Model) -> anyhow::Result<()> {
  clear_pws(model)?;

  let mut ncli = Nitrocli::new().model(model);

  // Fill slots 0 and 5
  let _ = ncli.handle(&["pws", "add", "--slot", "0", "name0", "login0", "pass0rd"])?;
  let _ = ncli.handle(&["pws", "add", "--slot", "5", "name5", "login5", "pass5rd"])?;

  // Try to add another one
  let out = ncli.handle(&["pws", "add", "name1", "login1", "passw1rd"])?;
  assert_eq!("Added PWS slot 1\n", out);

  assert_slot(model, 1, "name1", "login1", "passw1rd")?;

  Ok(())
}

#[test_device]
fn add_stdin(model: nitrokey::Model) -> anyhow::Result<()> {
  clear_pws(model)?;

  let mut ncli = Nitrocli::new().model(model);

  // Fill slots 0 and 5
  let _ = ncli.handle(&["pws", "add", "--slot", "0", "name0", "login0", "pass0rd"])?;
  let _ = ncli.handle(&["pws", "add", "--slot", "5", "name5", "login5", "pass5rd"])?;

  // Try to add another one
  let out = ncli.stdin("passw1rd").handle(&["pws", "add", "name1", "login1", "-"])?;
  assert_eq!("Added PWS slot 1\n", out);

  assert_slot(model, 1, "name1", "login1", "passw1rd")?;

  Ok(())
}

#[test_device]
fn update_stdin(model: nitrokey::Model) -> anyhow::Result<()> {
  clear_pws(model)?;

  let mut ncli = Nitrocli::new().model(model);

  let _ = ncli.handle(&["pws", "add", "--slot", "0", "name0", "login0", "pass0rd"])?;
  let _ = ncli.stdin("passw1rd").handle(&["pws", "update", "0", "--password", "-"])?;

  assert_slot(model, 0, "name0", "login0", "passw1rd")?;

  Ok(())
}
