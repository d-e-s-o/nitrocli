// otp.rs

// Copyright (C) 2019-2020 The Nitrocli Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use super::*;

use crate::args;

#[test_device]
fn set_invalid_slot_raw(model: nitrokey::Model) {
  let (rc, out, err) = Nitrocli::new()
    .model(model)
    .run(&["otp", "set", "100", "name", "1234", "-f", "hex"]);

  assert_ne!(rc, 0);
  assert_eq!(out, b"");
  assert_eq!(&err[..24], b"Failed to write OTP slot");
}

#[test_device]
fn set_invalid_slot(model: nitrokey::Model) {
  let err = Nitrocli::new()
    .model(model)
    .handle(&["otp", "set", "100", "name", "1234", "-f", "hex"])
    .unwrap_err()
    .to_string();

  assert_eq!(err, "Failed to write OTP slot");
}

#[test_device]
fn status(model: nitrokey::Model) -> anyhow::Result<()> {
  let re = regex::Regex::new(
    r#"^alg\tslot\tname
((totp|hotp)\t\d+\t.+\n)+$"#,
  )
  .unwrap();

  let mut ncli = Nitrocli::new().model(model);
  // Make sure that we have at least something to display by ensuring
  // that there is one slot programmed.
  let _ = ncli.handle(&["otp", "set", "0", "the-name", "123456", "-f", "hex"])?;

  let out = ncli.handle(&["otp", "status"])?;
  assert!(re.is_match(&out), out);
  Ok(())
}

#[test_device]
fn set_get_hotp(model: nitrokey::Model) -> anyhow::Result<()> {
  // Secret and expected HOTP values as per RFC 4226: Appendix D -- HOTP
  // Algorithm: Test Values.
  const SECRET: &str = "12345678901234567890";
  const OTP1: &str = concat!(755224, "\n");
  const OTP2: &str = concat!(287082, "\n");

  let mut ncli = Nitrocli::new().model(model);
  let _ = ncli.handle(&[
    "otp", "set", "-a", "hotp", "-f", "ascii", "1", "name", &SECRET,
  ])?;

  let out = ncli.handle(&["otp", "get", "-a", "hotp", "1"])?;
  assert_eq!(out, OTP1);

  let out = ncli.handle(&["otp", "get", "-a", "hotp", "1"])?;
  assert_eq!(out, OTP2);
  Ok(())
}

#[test_device]
fn set_get_totp(model: nitrokey::Model) -> anyhow::Result<()> {
  // Secret and expected TOTP values as per RFC 6238: Appendix B --
  // Test Vectors.
  const SECRET: &str = "12345678901234567890";
  const TIME: &str = stringify!(1111111111);
  const OTP: &str = concat!(14050471, "\n");

  let mut ncli = Nitrocli::new().model(model);
  let _ = ncli.handle(&["otp", "set", "-d", "8", "-f", "ascii", "2", "name", &SECRET])?;

  let out = ncli.handle(&["otp", "get", "-t", TIME, "2"])?;
  assert_eq!(out, OTP);
  Ok(())
}

#[test_device]
fn set_totp_uneven_chars(model: nitrokey::Model) -> anyhow::Result<()> {
  let secrets = [
    (args::OtpSecretFormat::Hex, "123"),
    (args::OtpSecretFormat::Base32, "FBILDWWGA2"),
  ];

  for (format, secret) in &secrets {
    let mut ncli = Nitrocli::new().model(model);
    let _ = ncli.handle(&["otp", "set", "-f", format.as_ref(), "3", "foobar", &secret])?;
  }
  Ok(())
}

#[test_device]
fn clear(model: nitrokey::Model) -> anyhow::Result<()> {
  let mut ncli = Nitrocli::new().model(model);
  let _ = ncli.handle(&["otp", "set", "3", "hotp-test", "abcdef"])?;
  let _ = ncli.handle(&["otp", "clear", "3"])?;
  let res = ncli.handle(&["otp", "get", "3"]);

  let err = res.unwrap_err().to_string();
  assert_eq!(err, "Failed to generate OTP");
  Ok(())
}
