// otp.rs

// *************************************************************************
// * Copyright (C) 2019-2020 Daniel Mueller (deso@posteo.net)              *
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

use super::*;

use crate::args;

#[test_device]
fn set_invalid_slot_raw(model: nitrokey::Model) {
  let (rc, out, err) = Nitrocli::with_model(model).run(&["otp", "set", "100", "name", "1234"]);

  assert_ne!(rc, 0);
  assert_eq!(out, b"");
  assert_eq!(&err[..24], b"Could not write OTP slot");
}

#[test_device]
fn set_invalid_slot(model: nitrokey::Model) {
  let err = Nitrocli::with_model(model)
    .handle(&["otp", "set", "100", "name", "1234"])
    .unwrap_err()
    .to_string();
  let expected = format!(
    "Could not write OTP slot: {}",
    nitrokey::Error::LibraryError(nitrokey::LibraryError::InvalidSlot)
  );

  assert_eq!(err, expected);
}

#[test_device]
fn status(model: nitrokey::Model) -> crate::Result<()> {
  let re = regex::Regex::new(
    r#"^alg\tslot\tname
((totp|hotp)\t\d+\t.+\n)+$"#,
  )
  .unwrap();

  let mut ncli = Nitrocli::with_model(model);
  // Make sure that we have at least something to display by ensuring
  // that there is one slot programmed.
  let _ = ncli.handle(&["otp", "set", "0", "the-name", "123456"])?;

  let out = ncli.handle(&["otp", "status"])?;
  assert!(re.is_match(&out), out);
  Ok(())
}

#[test_device]
fn set_get_hotp(model: nitrokey::Model) -> crate::Result<()> {
  // Secret and expected HOTP values as per RFC 4226: Appendix D -- HOTP
  // Algorithm: Test Values.
  const SECRET: &str = "12345678901234567890";
  const OTP1: &str = concat!(755224, "\n");
  const OTP2: &str = concat!(287082, "\n");

  let mut ncli = Nitrocli::with_model(model);
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
fn set_get_totp(model: nitrokey::Model) -> crate::Result<()> {
  // Secret and expected TOTP values as per RFC 6238: Appendix B --
  // Test Vectors.
  const SECRET: &str = "12345678901234567890";
  const TIME: &str = stringify!(1111111111);
  const OTP: &str = concat!(14050471, "\n");

  let mut ncli = Nitrocli::with_model(model);
  let _ = ncli.handle(&["otp", "set", "-d", "8", "-f", "ascii", "2", "name", &SECRET])?;

  let out = ncli.handle(&["otp", "get", "-t", TIME, "2"])?;
  assert_eq!(out, OTP);
  Ok(())
}

#[test_device]
fn set_totp_uneven_chars(model: nitrokey::Model) -> crate::Result<()> {
  let secrets = [
    (args::OtpSecretFormat::Hex, "123"),
    (args::OtpSecretFormat::Base32, "FBILDWWGA2"),
  ];

  for (format, secret) in &secrets {
    let mut ncli = Nitrocli::with_model(model);
    let _ = ncli.handle(&["otp", "set", "-f", format.as_ref(), "3", "foobar", &secret])?;
  }
  Ok(())
}

#[test_device]
fn clear(model: nitrokey::Model) -> crate::Result<()> {
  let mut ncli = Nitrocli::with_model(model);
  let _ = ncli.handle(&["otp", "set", "3", "hotp-test", "abcdef"])?;
  let _ = ncli.handle(&["otp", "clear", "3"])?;
  let res = ncli.handle(&["otp", "get", "3"]);

  let err = res.unwrap_err().to_string();
  let expected = format!(
    "Could not generate OTP: {}",
    nitrokey::Error::CommandError(nitrokey::CommandError::SlotNotProgrammed)
  );
  assert_eq!(err, expected);
  Ok(())
}
