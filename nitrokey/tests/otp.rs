// Copyright (C) 2018-2019 Robin Krahl <robin.krahl@ireas.org>
// SPDX-License-Identifier: MIT

mod util;

use std::fmt::Debug;
use std::ops::DerefMut;

use nitrokey::{
    Admin, Authenticate, CommandError, Config, ConfigureOtp, Device, GenerateOtp, LibraryError,
    OtpMode, OtpSlotData, DEFAULT_ADMIN_PIN, DEFAULT_USER_PIN,
};
use nitrokey_test::test as test_device;

// test suite according to RFC 4226, Appendix D
static HOTP_SECRET: &str = "3132333435363738393031323334353637383930";
static HOTP_CODES: &[&str] = &[
    "755224", "287082", "359152", "969429", "338314", "254676", "287922", "162583", "399871",
    "520489",
];

// test suite according to RFC 6238, Appendix B
static TOTP_SECRET: &str = "3132333435363738393031323334353637383930";
static TOTP_CODES: &[(u64, &[&str])] = &[
    (59, &["94287082", "37359152"]),
    (1111111109, &["07081804"]),
    (1111111111, &["14050471"]),
    (1234567890, &["89005924"]),
    (2000000000, &["69279037"]),
    (20000000000, &["65353130"]),
];

#[derive(PartialEq)]
enum TotpTimestampSize {
    U32,
    U64,
}

fn make_admin_test_device<'a, T>(device: T) -> Admin<'a, T>
where
    T: Device<'a>,
    (T, nitrokey::Error): Debug,
{
    unwrap_ok!(device.authenticate_admin(DEFAULT_ADMIN_PIN))
}

fn configure_hotp(admin: &mut ConfigureOtp, counter: u8) {
    let slot_data = OtpSlotData::new(1, "test-hotp", HOTP_SECRET, OtpMode::SixDigits);
    assert_ok!((), admin.write_hotp_slot(slot_data, counter.into()));
}

fn check_hotp_codes(device: &mut GenerateOtp, offset: u8) {
    HOTP_CODES.iter().enumerate().for_each(|(i, code)| {
        if i >= offset as usize {
            assert_ok!(code.to_string(), device.get_hotp_code(1));
        }
    });
}

#[test_device]
fn set_time(device: DeviceWrapper) {
    let mut device = device;
    assert_ok!((), device.set_time(1546385382, true));
    assert_ok!((), device.set_time(1546385392, false));
    assert_cmd_err!(CommandError::Timestamp, device.set_time(1546385292, false));
    assert_ok!((), device.set_time(1546385382, true));
}

#[test_device]
fn hotp_no_pin(device: DeviceWrapper) {
    let mut admin = make_admin_test_device(device);
    let config = Config::new(None, None, None, false);
    assert_ok!((), admin.write_config(config));

    configure_hotp(&mut admin, 0);
    check_hotp_codes(admin.deref_mut(), 0);

    configure_hotp(&mut admin, 5);
    check_hotp_codes(admin.deref_mut(), 5);

    configure_hotp(&mut admin, 0);
    check_hotp_codes(&mut admin.device(), 0);
}

#[test_device]
fn hotp_pin(device: DeviceWrapper) {
    let mut admin = make_admin_test_device(device);
    let config = Config::new(None, None, None, true);
    assert_ok!((), admin.write_config(config));

    configure_hotp(&mut admin, 0);
    let mut user = unwrap_ok!(admin.device().authenticate_user(DEFAULT_USER_PIN));
    check_hotp_codes(&mut user, 0);

    assert_cmd_err!(CommandError::NotAuthorized, user.device().get_hotp_code(1));
}

#[test_device]
fn hotp_slot_name(device: DeviceWrapper) {
    let mut admin = make_admin_test_device(device);
    let slot_data = OtpSlotData::new(1, "test-hotp", HOTP_SECRET, OtpMode::SixDigits);
    assert_ok!((), admin.write_hotp_slot(slot_data, 0));

    let device = admin.device();
    assert_ok!("test-hotp".to_string(), device.get_hotp_slot_name(1));
    assert_lib_err!(LibraryError::InvalidSlot, device.get_hotp_slot_name(4));
}

#[test_device]
fn hotp_error(device: DeviceWrapper) {
    let mut admin = make_admin_test_device(device);
    let slot_data = OtpSlotData::new(1, "", HOTP_SECRET, OtpMode::SixDigits);
    assert_cmd_err!(CommandError::NoName, admin.write_hotp_slot(slot_data, 0));
    let slot_data = OtpSlotData::new(4, "test", HOTP_SECRET, OtpMode::SixDigits);
    assert_lib_err!(
        LibraryError::InvalidSlot,
        admin.write_hotp_slot(slot_data, 0)
    );
    let slot_data = OtpSlotData::new(1, "test", "foobar", OtpMode::SixDigits);
    assert_lib_err!(
        LibraryError::InvalidHexString,
        admin.write_hotp_slot(slot_data, 0)
    );
    let code = admin.get_hotp_code(4);
    assert_lib_err!(LibraryError::InvalidSlot, code);
}

#[test_device]
fn hotp_erase(device: DeviceWrapper) {
    let mut admin = make_admin_test_device(device);
    let config = Config::new(None, None, None, false);
    assert_ok!((), admin.write_config(config));
    let slot_data = OtpSlotData::new(1, "test1", HOTP_SECRET, OtpMode::SixDigits);
    assert_ok!((), admin.write_hotp_slot(slot_data, 0));
    let slot_data = OtpSlotData::new(2, "test2", HOTP_SECRET, OtpMode::SixDigits);
    assert_ok!((), admin.write_hotp_slot(slot_data, 0));

    assert_ok!((), admin.erase_hotp_slot(1));

    let mut device = admin.device();
    let result = device.get_hotp_slot_name(1);
    assert_cmd_err!(CommandError::SlotNotProgrammed, result);
    let result = device.get_hotp_code(1);
    assert_cmd_err!(CommandError::SlotNotProgrammed, result);

    assert_ok!("test2".to_string(), device.get_hotp_slot_name(2));
}

fn configure_totp(admin: &mut ConfigureOtp, factor: u64) {
    let slot_data = OtpSlotData::new(1, "test-totp", TOTP_SECRET, OtpMode::EightDigits);
    let time_window = 30u64.checked_mul(factor).unwrap();
    assert_ok!((), admin.write_totp_slot(slot_data, time_window as u16));
}

fn check_totp_codes(device: &mut GenerateOtp, factor: u64, timestamp_size: TotpTimestampSize) {
    for (base_time, codes) in TOTP_CODES {
        let time = base_time.checked_mul(factor).unwrap();
        let is_u64 = time > u32::max_value() as u64;
        if is_u64 != (timestamp_size == TotpTimestampSize::U64) {
            continue;
        }

        assert_ok!((), device.set_time(time, true));
        let code = unwrap_ok!(device.get_totp_code(1));
        assert!(
            code.contains(&code),
            "Generated TOTP code {} for {}, but expected one of {}",
            code,
            base_time,
            codes.join(", ")
        );
    }
}

#[test_device]
fn totp_no_pin(device: DeviceWrapper) {
    let mut admin = make_admin_test_device(device);
    let config = Config::new(None, None, None, false);
    assert_ok!((), admin.write_config(config));

    configure_totp(&mut admin, 1);
    check_totp_codes(admin.deref_mut(), 1, TotpTimestampSize::U32);

    configure_totp(&mut admin, 2);
    check_totp_codes(admin.deref_mut(), 2, TotpTimestampSize::U32);

    configure_totp(&mut admin, 1);
    check_totp_codes(&mut admin.device(), 1, TotpTimestampSize::U32);
}

#[test_device]
// Nitrokey Storage does only support timestamps that fit in a 32-bit
// unsigned integer, so don't test with it.
fn totp_no_pin_64(device: Pro) {
    let mut admin = make_admin_test_device(device);
    let config = Config::new(None, None, None, false);
    assert_ok!((), admin.write_config(config));

    configure_totp(&mut admin, 1);
    check_totp_codes(admin.deref_mut(), 1, TotpTimestampSize::U64);

    configure_totp(&mut admin, 2);
    check_totp_codes(admin.deref_mut(), 2, TotpTimestampSize::U64);

    configure_totp(&mut admin, 1);
    check_totp_codes(&mut admin.device(), 1, TotpTimestampSize::U64);
}

#[test_device]
fn totp_pin(device: DeviceWrapper) {
    let mut admin = make_admin_test_device(device);
    let config = Config::new(None, None, None, true);
    assert_ok!((), admin.write_config(config));

    configure_totp(&mut admin, 1);
    let mut user = unwrap_ok!(admin.device().authenticate_user(DEFAULT_USER_PIN));
    check_totp_codes(&mut user, 1, TotpTimestampSize::U32);

    assert_cmd_err!(CommandError::NotAuthorized, user.device().get_totp_code(1));
}

#[test_device]
// See comment for totp_no_pin_64.
fn totp_pin_64(device: Pro) {
    let mut admin = make_admin_test_device(device);
    let config = Config::new(None, None, None, true);
    assert_ok!((), admin.write_config(config));

    configure_totp(&mut admin, 1);
    let mut user = unwrap_ok!(admin.device().authenticate_user(DEFAULT_USER_PIN));
    check_totp_codes(&mut user, 1, TotpTimestampSize::U64);

    assert_cmd_err!(CommandError::NotAuthorized, user.device().get_totp_code(1));
}

#[test_device]
fn totp_slot_name(device: DeviceWrapper) {
    let mut admin = make_admin_test_device(device);
    let slot_data = OtpSlotData::new(1, "test-totp", TOTP_SECRET, OtpMode::EightDigits);
    assert_ok!((), admin.write_totp_slot(slot_data, 0));

    let device = admin.device();
    let result = device.get_totp_slot_name(1);
    assert_ok!("test-totp", result);
    let result = device.get_totp_slot_name(16);
    assert_lib_err!(LibraryError::InvalidSlot, result);
}

#[test_device]
fn totp_error(device: DeviceWrapper) {
    let mut admin = make_admin_test_device(device);
    let slot_data = OtpSlotData::new(1, "", TOTP_SECRET, OtpMode::SixDigits);
    assert_cmd_err!(CommandError::NoName, admin.write_totp_slot(slot_data, 0));
    let slot_data = OtpSlotData::new(20, "test", TOTP_SECRET, OtpMode::SixDigits);
    assert_lib_err!(
        LibraryError::InvalidSlot,
        admin.write_totp_slot(slot_data, 0)
    );
    let slot_data = OtpSlotData::new(4, "test", "foobar", OtpMode::SixDigits);
    assert_lib_err!(
        LibraryError::InvalidHexString,
        admin.write_totp_slot(slot_data, 0)
    );
    let code = admin.get_totp_code(20);
    assert_lib_err!(LibraryError::InvalidSlot, code);
}

#[test_device]
fn totp_erase(device: DeviceWrapper) {
    let mut admin = make_admin_test_device(device);
    let config = Config::new(None, None, None, false);
    assert_ok!((), admin.write_config(config));
    let slot_data = OtpSlotData::new(1, "test1", TOTP_SECRET, OtpMode::SixDigits);
    assert_ok!((), admin.write_totp_slot(slot_data, 0));
    let slot_data = OtpSlotData::new(2, "test2", TOTP_SECRET, OtpMode::SixDigits);
    assert_ok!((), admin.write_totp_slot(slot_data, 0));

    assert_ok!((), admin.erase_totp_slot(1));

    let device = admin.device();
    let result = device.get_totp_slot_name(1);
    assert_cmd_err!(CommandError::SlotNotProgrammed, result);
    let result = device.get_totp_code(1);
    assert_cmd_err!(CommandError::SlotNotProgrammed, result);

    assert_ok!("test2".to_string(), device.get_totp_slot_name(2));
}
