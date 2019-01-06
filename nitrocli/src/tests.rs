// tests.rs

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

use nitrokey_test::test as test_device;

trait IntoArg {
  fn into_arg(self) -> &'static str;
}

impl IntoArg for nitrokey::Pro {
  fn into_arg(self) -> &'static str {
    "--model=pro"
  }
}

impl IntoArg for nitrokey::Storage {
  fn into_arg(self) -> &'static str {
    "--model=storage"
  }
}

impl IntoArg for nitrokey::DeviceWrapper {
  fn into_arg(self) -> &'static str {
    match self {
      nitrokey::DeviceWrapper::Pro(x) => x.into_arg(),
      nitrokey::DeviceWrapper::Storage(x) => x.into_arg(),
    }
  }
}

impl IntoArg for &'static str {
  fn into_arg(self) -> &'static str {
    self
  }
}

/// Run `nitrocli` with the given set of arguments.
fn nitrocli<D>(device: D, args: &[&'static str]) -> crate::Result<Vec<u8>>
where
  D: IntoArg,
{
  let args = ["nitrocli", device.into_arg()]
    .into_iter()
    .chain(args)
    .cloned()
    .map(|x| x.to_owned())
    .collect();

  let mut stdout = Vec::new();
  let mut stderr = Vec::new();

  crate::args::handle_arguments(args, &mut stdout, &mut stderr).map(|_| stdout)
}

static SECRET: &'static str = "3132333435363738393031323334353637383930";

#[test_device]
fn totp_no_pin(device: nitrokey::DeviceWrapper) -> crate::Result<()> {
  let device = device.into_arg();
  // TODO: This call can potentially ask for the admin PIN which makes it
  //       interactive and unacceptable.
  let _ = nitrocli(device, &["config", "set", "--no-otp-pin"])?;

  let slot = "0";
  let name = "test-totp";

  let _ = nitrocli(
    device,
    &["otp", "set", slot, name, SECRET, "--algorithm", "totp"],
  )?;

  let output = nitrocli(
    device,
    &[
      "otp",
      "get",
      slot,
      "--algorithm",
      "totp",
      "--time",
      "1111111111",
    ],
  )?;
  assert_eq!(output, b"050471\n");

  let _ = nitrocli(
    device,
    &[
      "otp",
      "set",
      slot,
      name,
      SECRET,
      "--algorithm",
      "totp",
      "--digits",
      "8",
    ],
  )?;

  let output = nitrocli(
    device,
    &[
      "otp",
      "get",
      slot,
      "--algorithm",
      "totp",
      "--time",
      "1111111111",
    ],
  )?;
  assert_eq!(output, b"14050471\n");

  Ok(())
}
