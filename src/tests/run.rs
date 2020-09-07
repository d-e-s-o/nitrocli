// run.rs

// Copyright (C) 2019-2020 The Nitrocli Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections;
use std::path;

use super::*;

#[test]
fn no_command_or_option() {
  let (rc, out, err) = Nitrocli::new().run(&[]);

  assert_ne!(rc, 0);
  assert_eq!(out, b"");

  let s = String::from_utf8_lossy(&err).into_owned();
  assert!(s.starts_with("nitrocli"), s);
  assert!(s.contains("USAGE:\n"), s);
}

#[test]
fn help_options() {
  fn test_run(args: &[&str], help: &str) {
    let mut all = args.to_vec();
    all.push(help);

    let (rc, out, err) = Nitrocli::new().run(&all);

    assert_eq!(rc, 0);
    assert_eq!(err, b"");

    let s = String::from_utf8_lossy(&out).into_owned();
    let mut args = args.to_vec();
    args.insert(0, "nitrocli");
    assert!(s.starts_with(&args.join("-")), s);
    assert!(s.contains("USAGE:\n"), s);
  }

  fn test(args: &[&str]) {
    test_run(args, "--help");
    test_run(args, "-h");
  }

  test(&[]);
  test(&["config"]);
  test(&["config", "get"]);
  test(&["config", "set"]);
  test(&["encrypted"]);
  test(&["encrypted", "open"]);
  test(&["encrypted", "close"]);
  test(&["hidden"]);
  test(&["hidden", "close"]);
  test(&["hidden", "create"]);
  test(&["hidden", "open"]);
  test(&["lock"]);
  test(&["otp"]);
  test(&["otp", "clear"]);
  test(&["otp", "get"]);
  test(&["otp", "set"]);
  test(&["otp", "status"]);
  test(&["pin"]);
  test(&["pin", "clear"]);
  test(&["pin", "set"]);
  test(&["pin", "unblock"]);
  test(&["pws"]);
  test(&["pws", "clear"]);
  test(&["pws", "get"]);
  test(&["pws", "set"]);
  test(&["pws", "status"]);
  test(&["reset"]);
  test(&["status"]);
  test(&["unencrypted"]);
  test(&["unencrypted", "set"]);
}

#[test]
#[ignore]
fn version_option() {
  // clap sends the version output directly to stdout: https://github.com/clap-rs/clap/issues/1390
  // Therefore we ignore this test for the time being.

  fn test(re: &regex::Regex, opt: &'static str) {
    let (rc, out, err) = Nitrocli::new().run(&[opt]);

    assert_eq!(rc, 0);
    assert_eq!(err, b"");

    let s = String::from_utf8_lossy(&out).into_owned();
    let _ = re;
    assert!(re.is_match(&s), out);
  }

  let re = regex::Regex::new(r"^nitrocli \d+.\d+.\d+(-[^-]+)*\n$").unwrap();

  test(&re, "--version");
  test(&re, "-V");
}

#[test]
fn config_file() {
  let config =
    crate::config::read_config_file(&path::Path::new("doc/config.example.toml")).unwrap();

  assert_eq!(Some(crate::args::DeviceModel::Pro), config.model);
  assert_eq!(true, config.no_cache);
  assert_eq!(2, config.verbosity);
}

#[test_device]
fn connect_multiple(_model: nitrokey::Model) -> anyhow::Result<()> {
  let devices = nitrokey::list_devices()?;
  if devices.len() > 1 {
    let res = Nitrocli::new().handle(&["status"]);
    let err = res.unwrap_err().to_string();
    assert_eq!(
      err,
      "Multiple Nitrokey devices found.  Use the --model and --serial-number options to select one"
    );
  }
  Ok(())
}

#[test_device]
fn connect_serial_number(_model: nitrokey::Model) -> anyhow::Result<()> {
  let devices = nitrokey::list_devices()?;
  for serial_number in devices.iter().filter_map(|d| d.serial_number) {
    let res = Nitrocli::new().handle(&["status", &format!("--serial-number={}", serial_number)])?;
    assert!(res.contains(&format!("serial number:     {}\n", serial_number)));
  }
  Ok(())
}

#[test_device]
fn connect_wrong_serial_number(_model: nitrokey::Model) {
  let res = Nitrocli::new().handle(&["status", "--serial-number=0xdeadbeef"]);
  let err = res.unwrap_err().to_string();
  assert_eq!(
    err,
    "Nitrokey device not found (filter: serial number in [0xdeadbeef])"
  );
}

#[test_device]
fn connect_model(_model: nitrokey::Model) -> anyhow::Result<()> {
  let devices = nitrokey::list_devices()?;
  let mut model_counts = collections::BTreeMap::new();
  let _ = model_counts.insert(nitrokey::Model::Pro.to_string(), 0);
  let _ = model_counts.insert(nitrokey::Model::Storage.to_string(), 0);
  for model in devices.iter().filter_map(|d| d.model) {
    *model_counts.entry(model.to_string()).or_default() += 1;
  }

  for (model, count) in model_counts {
    let res = Nitrocli::new().handle(&["status", &format!("--model={}", model.to_lowercase())]);
    if count == 0 {
      let err = res.unwrap_err().to_string();
      assert_eq!(
        err,
        format!(
          "Nitrokey device not found (filter: model={})",
          model.to_lowercase()
        )
      );
    } else if count == 1 {
      assert!(res?.contains(&format!("model:             {}\n", model)));
    } else {
      let err = res.unwrap_err().to_string();
      assert_eq!(
        err,
        format!(
          "Multiple Nitrokey devices found (filter: model={}).  ",
          model.to_lowercase()
        ) + "Use the --model and --serial-number options to select one"
      );
    }
  }

  Ok(())
}
