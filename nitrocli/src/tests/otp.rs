// otp.rs

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

use super::*;

use crate::args;

#[test_device]
fn set_invalid_slot_raw(device: nitrokey::DeviceWrapper) {
  let (rc, out, err) = Nitrocli::with_dev(device).run(&["otp", "set", "100", "name", "1234"]);

  assert_ne!(rc, 0);
  assert_eq!(out, b"");
  assert_eq!(&err[..24], b"Could not write OTP slot");
}

#[test_device]
fn set_invalid_slot(device: nitrokey::DeviceWrapper) {
  let res = Nitrocli::with_dev(device).handle(&["otp", "set", "100", "name", "1234"]);

  assert_eq!(
    res.unwrap_lib_err(),
    (
      Some("Could not write OTP slot"),
      nitrokey::LibraryError::InvalidSlot
    )
  );
}

#[test_device]
fn status(device: nitrokey::DeviceWrapper) -> crate::Result<()> {
  let re = regex::Regex::new(
    r#"^alg\tslot\tname
((totp|hotp)\t\d+\t.+\n)+$"#,
  )
  .unwrap();

  let mut ncli = Nitrocli::with_dev(device);
  // Make sure that we have at least something to display by ensuring
  // that there is one slot programmed.
  let _ = ncli.handle(&["otp", "set", "0", "the-name", "123456"])?;

  let out = ncli.handle(&["otp", "status"])?;
  assert!(re.is_match(&out), out);
  Ok(())
}

#[test_device]
fn set_get_hotp(device: nitrokey::DeviceWrapper) -> crate::Result<()> {
  // Secret and expected HOTP values as per RFC 4226: Appendix D -- HOTP
  // Algorithm: Test Values.
  const SECRET: &str = "12345678901234567890";
  const OTP1: &str = concat!(755224, "\n");
  const OTP2: &str = concat!(287082, "\n");

  let mut ncli = Nitrocli::with_dev(device);
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
fn set_get_totp(device: nitrokey::DeviceWrapper) -> crate::Result<()> {
  // Secret and expected TOTP values as per RFC 6238: Appendix B --
  // Test Vectors.
  const SECRET: &str = "12345678901234567890";
  const TIME: &str = stringify!(1111111111);
  const OTP: &str = concat!(14050471, "\n");

  let mut ncli = Nitrocli::with_dev(device);
  let _ = ncli.handle(&["otp", "set", "-d", "8", "-f", "ascii", "2", "name", &SECRET])?;

  let out = ncli.handle(&["otp", "get", "-t", TIME, "2"])?;
  assert_eq!(out, OTP);
  Ok(())
}

#[test_device]
fn set_totp_uneven_chars(device: nitrokey::DeviceWrapper) -> crate::Result<()> {
  let secrets = [
    (args::OtpSecretFormat::Hex, "123"),
    (args::OtpSecretFormat::Base32, "FBILDWWGA2"),
  ];

  let mut ncli = Nitrocli::with_dev(device);
  for (format, secret) in &secrets {
    let _ = ncli.handle(&["otp", "set", "-f", format.as_ref(), "3", "foobar", &secret])?;
  }
  Ok(())
}

#[test_device]
fn clear(device: nitrokey::DeviceWrapper) -> crate::Result<()> {
  let mut ncli = Nitrocli::with_dev(device);
  let _ = ncli.handle(&["otp", "set", "3", "hotp-test", "abcdef"])?;
  let _ = ncli.handle(&["otp", "clear", "3"])?;
  let res = ncli.handle(&["otp", "get", "3"]);

  assert_eq!(
    res.unwrap_cmd_err(),
    (
      Some("Could not generate OTP"),
      nitrokey::CommandError::SlotNotProgrammed
    )
  );
  Ok(())
}
