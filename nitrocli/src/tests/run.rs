// run.rs

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

use std::fs;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path;

use super::*;

#[test]
fn no_command_or_option() {
  let (rc, out, err) = Nitrocli::new().run(&[]);

  assert_ne!(rc, 0);
  assert_eq!(out, b"");

  let s = String::from_utf8_lossy(&err).into_owned();
  assert!(s.starts_with("Usage:\n"), s);
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
    let expected = format!("Usage:\n  nitrocli {}", args.join(" "));
    assert!(s.starts_with(&expected), s);
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
fn version_option() {
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
fn extension() -> crate::Result<()> {
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
  let out = Nitrocli::make().path(path).build().handle(&["ext"])?;
  assert_eq!(out, "success\n");
  Ok(())
}

#[test]
fn extension_failure() -> crate::Result<()> {
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
  let err = Nitrocli::make()
    .path(path)
    .build()
    .handle(&["ext"])
    .unwrap_err();

  match err {
    crate::Error::ExtensionFailed(ext, rc) => {
      assert_eq!(&ext, "ext");
      assert_eq!(rc, Some(42));
    }
    _ => panic!("Unexpected error variant found: {:?}", err),
  };
  Ok(())
}

#[test_device]
fn extension_arguments(model: nitrokey::Model) -> crate::Result<()> {
  fn test<F>(model: nitrokey::Model, what: &str, args: &[&str], check: F) -> crate::Result<()>
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

      ext.write_all(include_bytes!("extension_model_test.py"))?;
    }

    let mut args = args.to_vec();
    args.append(&mut vec!["ext", what]);

    let path = ext_dir.path().as_os_str().to_os_string();
    let out = Nitrocli::make()
      .model(model)
      .path(path)
      .build()
      .handle(&args)?;

    assert!(check(&out), out);
    Ok(())
  }

  test(model, "model", &[], |out| {
    out == model.to_string().to_lowercase() + "\n"
  })?;
  test(model, "nitrocli", &[], |out| {
    path::Path::new(out)
      .file_stem()
      .unwrap()
      .to_str()
      .unwrap()
      .trim()
      .contains("nitrocli")
  })?;
  test(model, "verbosity", &[], |out| out == "0\n")?;
  test(model, "verbosity", &["-v"], |out| out == "1\n")?;
  test(model, "verbosity", &["-v", "--verbose"], |out| out == "2\n")?;
  Ok(())
}
