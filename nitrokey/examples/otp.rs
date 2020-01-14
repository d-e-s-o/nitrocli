// Copyright (C) 2020 Robin Krahl <robin.krahl@ireas.org>
// SPDX-License-Identifier: CC0-1.0

//! Connects to a Nitrokey device, configures an TOTP slot and generates a one-time password from
//! it.

use std::time;

use nitrokey::{Authenticate, ConfigureOtp, Device, GenerateOtp};

fn main() -> Result<(), nitrokey::Error> {
    let mut manager = nitrokey::take()?;
    let device = manager.connect()?;

    // Configure the OTP slot (requires admin PIN)
    let data = nitrokey::OtpSlotData::new(
        1,
        "test",
        "3132333435363738393031323334353637383930",
        nitrokey::OtpMode::SixDigits,
    );
    let mut admin = device.authenticate_admin("12345678")?;
    admin.write_totp_slot(data, 30)?;
    let mut device = admin.device();

    // Set the time for the OTP generation
    let time = time::SystemTime::now()
        .duration_since(time::UNIX_EPOCH)
        .expect("Invalid system time");
    device.set_time(time.as_secs(), true)?;

    // Generate a one-time password -- depending on the configuration, we have to set the user PIN
    let config = device.get_config()?;
    let otp = if config.user_password {
        let user = device.authenticate_user("123456")?;
        user.get_totp_code(1)
    } else {
        device.get_totp_code(1)
    }?;
    println!("Generated OTP code: {}", otp);

    Ok(())
}
