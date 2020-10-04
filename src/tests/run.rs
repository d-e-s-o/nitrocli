// run.rs

// Copyright (C) 2019-2020 The Nitrocli Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections;
use std::fs;
use std::io::Write;
use std::ops;
use std::os::unix::fs::OpenOptionsExt;
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
      "Multiple Nitrokey devices found.  Use the --model, --serial-number, and --usb-path options to select one"
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
fn connect_usb_path(_model: nitrokey::Model) -> anyhow::Result<()> {
  for device in nitrokey::list_devices()? {
    let res = Nitrocli::new().handle(&["status", &format!("--usb-path={}", device.path)]);
    assert!(res.is_ok());
    let res = res?;
    if let Some(model) = device.model {
      assert!(res.contains(&format!("model:             {}\n", model)));
    }
    if let Some(sn) = device.serial_number {
      assert!(res.contains(&format!("serial number:     {}\n", sn)));
    }
  }
  Ok(())
}

#[test_device]
fn connect_wrong_usb_path(_model: nitrokey::Model) {
  let res = Nitrocli::new().handle(&["status", "--usb-path=not-a-path"]);
  let err = res.unwrap_err().to_string();
  assert_eq!(
    err,
    "Nitrokey device not found (filter: usb path=not-a-path)"
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
        ) + "Use the --model, --serial-number, and --usb-path options to select one"
      );
    }
  }

  Ok(())
}

#[test_device]
fn connect_usb_path_model_serial(_model: nitrokey::Model) -> anyhow::Result<()> {
  let devices = nitrokey::list_devices()?;
  for device in devices {
    let mut args = Vec::new();
    args.push("status".to_owned());
    args.push(format!("--usb-path={}", device.path));
    if let Some(model) = device.model {
      args.push(format!("--model={}", model.to_string().to_lowercase()));
    }
    if let Some(sn) = device.serial_number {
      args.push(format!("--serial-number={}", sn));
    }

    let res = Nitrocli::new().handle(&args.iter().map(ops::Deref::deref).collect::<Vec<_>>())?;
    if let Some(model) = device.model {
      assert!(res.contains(&format!("model:             {}\n", model)));
    }
    if let Some(sn) = device.serial_number {
      assert!(res.contains(&format!("serial number:     {}\n", sn)));
    }
  }
  Ok(())
}

#[test_device]
fn connect_usb_path_model_wrong_serial(_model: nitrokey::Model) -> anyhow::Result<()> {
  let devices = nitrokey::list_devices()?;
  for device in devices {
    let mut args = Vec::new();
    args.push("status".to_owned());
    args.push(format!("--usb-path={}", device.path));
    if let Some(model) = device.model {
      args.push(format!("--model={}", model.to_string().to_lowercase()));
    }
    args.push("--serial-number=0xdeadbeef".to_owned());

    let res = Nitrocli::new().handle(&args.iter().map(ops::Deref::deref).collect::<Vec<_>>());
    let err = res.unwrap_err().to_string();
    if let Some(model) = device.model {
      assert_eq!(
        err,
        format!(
          "Nitrokey device not found (filter: model={}, serial number in [0xdeadbeef], usb path={})",
          model.to_string().to_lowercase(),
          device.path
        )
      );
    } else {
      assert_eq!(
        err,
        format!(
          "Nitrokey device not found (filter: serial number in [0xdeadbeef], usb path={})",
          device.path
        )
      );
    }
  }
  Ok(())
}

#[test]
fn extension() -> anyhow::Result<()> {
  let ext_dir = tempfile::tempdir()?;
  {
    let mut ext = fs::OpenOptions::new()
      .create(true)
      .mode(0o755)
      .write(true)
      .open(ext_dir.path().join("nitrocli-ext"))?;

    ext.write_all(
      br#"#!/usr/bin/env python
print("success")
"#,
    )?;
  }

  let path = ext_dir.path().as_os_str().to_os_string();
  // Make sure that the extension appears in the help text.
  let out = Nitrocli::new().path(&path).handle(&["--help"])?;
  assert!(out.contains("ext            Run the ext extension\n"), out);
  // And, of course, that we can invoke it.
  let out = Nitrocli::new().path(&path).handle(&["ext"])?;
  assert_eq!(out, "success\n");
  Ok(())
}

#[test]
fn extension_failure() -> anyhow::Result<()> {
  let ext_dir = tempfile::tempdir()?;
  {
    let mut ext = fs::OpenOptions::new()
      .create(true)
      .mode(0o755)
      .write(true)
      .open(ext_dir.path().join("nitrocli-ext"))?;

    ext.write_all(
      br#"#!/usr/bin/env python
import sys
sys.exit(42);
"#,
    )?;
  }

  let path = ext_dir.path().as_os_str().to_os_string();
  let mut ncli = Nitrocli::new().path(path);

  let err = ncli.handle(&["ext"]).unwrap_err();
  // The extension is responsible for printing any error messages.
  // Nitrocli is expected not to mess with them, including adding
  // additional information.
  if let Some(crate::DirectExitError(rc)) = err.downcast_ref::<crate::DirectExitError>() {
    assert_eq!(*rc, 42)
  } else {
    panic!("encountered unexpected error: {:#}", err)
  }

  let (rc, out, err) = ncli.run(&["ext"]);
  assert_eq!(rc, 42);
  assert_eq!(out, b"");
  assert_eq!(err, b"");
  Ok(())
}

#[test_device]
fn extension_arguments(model: nitrokey::Model) -> anyhow::Result<()> {
  fn test<F>(model: nitrokey::Model, what: &str, args: &[&str], check: F) -> anyhow::Result<()>
  where
    F: FnOnce(&str) -> bool,
  {
    let ext_dir = tempfile::tempdir()?;
    {
      let mut ext = fs::OpenOptions::new()
        .create(true)
        .mode(0o755)
        .write(true)
        .open(ext_dir.path().join("nitrocli-ext"))?;

      ext.write_all(include_bytes!("extension_var_test.py"))?;
    }

    let mut args = args.to_vec();
    args.append(&mut vec!["ext", what]);

    let path = ext_dir.path().as_os_str().to_os_string();
    let out = Nitrocli::new().model(model).path(path).handle(&args)?;

    assert!(check(&out), out);
    Ok(())
  }

  test(model, "binary", &[], |out| {
    path::Path::new(out)
      .file_stem()
      .unwrap()
      .to_str()
      .unwrap()
      .trim()
      .contains("nitrocli")
  })?;
  test(model, "model", &[], |out| {
    out == model.to_string().to_lowercase() + "\n"
  })?;
  test(model, "no-cache", &[], |out| out == "true\n")?;
  test(model, "serial-numbers", &[], |out| out == "\n")?;
  test(model, "verbosity", &[], |out| out == "0\n")?;
  test(model, "verbosity", &["-v"], |out| out == "1\n")?;
  test(model, "verbosity", &["-v", "--verbose"], |out| out == "2\n")?;

  // NITROCLI_USB_PATH should not be set, so the program errors out.
  let _ = test(model, "usb-path", &[], |out| out == "\n").unwrap_err();
  Ok(())
}
