mod util;

use std::ops::Deref;

use nitrokey::{
    Admin, Authenticate, CommandError, Config, ConfigureOtp, GenerateOtp, OtpMode, OtpSlotData,
};

use crate::util::{Target, ADMIN_PASSWORD, USER_PASSWORD};

// test suite according to RFC 4226, Appendix D
static HOTP_SECRET: &str = "3132333435363738393031323334353637383930";
static HOTP_CODES: &[&str] = &[
    "755224", "287082", "359152", "969429", "338314", "254676", "287922", "162583", "399871",
    "520489",
];

// test suite according to RFC 6238, Appendix B
static TOTP_SECRET: &str = "3132333435363738393031323334353637383930";
static TOTP_CODES: &[(u64, &str)] = &[
    (59, "94287082"),
    (1111111109, "07081804"),
    (1111111111, "14050471"),
    (1234567890, "89005924"),
    (2000000000, "69279037"),
    (20000000000, "65353130"),
];

#[derive(PartialEq)]
enum TotpTimestampSize {
    U32,
    U64,
}

fn get_admin_test_device() -> Admin<Target> {
    Target::connect()
        .expect("Could not connect to the Nitrokey.")
        .authenticate_admin(ADMIN_PASSWORD)
        .expect("Could not login as admin.")
}

fn configure_hotp(admin: &ConfigureOtp, counter: u8) {
    let slot_data = OtpSlotData::new(1, "test-hotp", HOTP_SECRET, OtpMode::SixDigits);
    assert!(admin.write_hotp_slot(slot_data, counter.into()).is_ok());
}

fn check_hotp_codes(device: &GenerateOtp, offset: u8) {
    HOTP_CODES.iter().enumerate().for_each(|(i, code)| {
        if i >= offset as usize {
            let result = device.get_hotp_code(1);
            assert_eq!(code, &result.unwrap());
        }
    });
}

#[test]
#[cfg_attr(not(any(feature = "test-pro", feature = "test-storage")), ignore)]
fn set_time() {
    let device = Target::connect().expect("Could not connect to the Nitrokey.");
    assert_eq!(Ok(()), device.set_time(1546385382, true));
    assert_eq!(Ok(()), device.set_time(1546385392, false));
    assert_eq!(Err(CommandError::Timestamp), device.set_time(1546385292, false));
    assert_eq!(Ok(()), device.set_time(1546385382, true));
}

#[test]
#[cfg_attr(not(any(feature = "test-pro", feature = "test-storage")), ignore)]
fn hotp_no_pin() {
    let admin = get_admin_test_device();
    let config = Config::new(None, None, None, false);
    assert!(admin.write_config(config).is_ok());

    configure_hotp(&admin, 0);
    check_hotp_codes(admin.deref(), 0);

    configure_hotp(&admin, 5);
    check_hotp_codes(admin.deref(), 5);

    configure_hotp(&admin, 0);
    check_hotp_codes(&admin.device(), 0);
}

#[test]
#[cfg_attr(not(any(feature = "test-pro", feature = "test-storage")), ignore)]
fn hotp_pin() {
    let admin = get_admin_test_device();
    let config = Config::new(None, None, None, true);
    assert!(admin.write_config(config).is_ok());

    configure_hotp(&admin, 0);
    let user = admin.device().authenticate_user(USER_PASSWORD).unwrap();
    check_hotp_codes(&user, 0);

    assert!(user.device().get_hotp_code(1).is_err());
}

#[test]
#[cfg_attr(not(any(feature = "test-pro", feature = "test-storage")), ignore)]
fn hotp_slot_name() {
    let admin = get_admin_test_device();
    let slot_data = OtpSlotData::new(1, "test-hotp", HOTP_SECRET, OtpMode::SixDigits);
    assert!(admin.write_hotp_slot(slot_data, 0).is_ok());

    let device = admin.device();
    let result = device.get_hotp_slot_name(1);
    assert_eq!("test-hotp", result.unwrap());
    let result = device.get_hotp_slot_name(4);
    assert_eq!(CommandError::InvalidSlot, result.unwrap_err());
}

#[test]
#[cfg_attr(not(any(feature = "test-pro", feature = "test-storage")), ignore)]
fn hotp_error() {
    let admin = get_admin_test_device();
    let slot_data = OtpSlotData::new(1, "", HOTP_SECRET, OtpMode::SixDigits);
    assert_eq!(
        Err(CommandError::NoName),
        admin.write_hotp_slot(slot_data, 0)
    );
    let slot_data = OtpSlotData::new(4, "test", HOTP_SECRET, OtpMode::SixDigits);
    assert_eq!(
        Err(CommandError::InvalidSlot),
        admin.write_hotp_slot(slot_data, 0)
    );
    let code = admin.get_hotp_code(4);
    assert_eq!(CommandError::InvalidSlot, code.unwrap_err());
}

#[test]
#[cfg_attr(not(any(feature = "test-pro", feature = "test-storage")), ignore)]
fn hotp_erase() {
    let admin = get_admin_test_device();
    let config = Config::new(None, None, None, false);
    assert!(admin.write_config(config).is_ok());
    let slot_data = OtpSlotData::new(1, "test1", HOTP_SECRET, OtpMode::SixDigits);
    assert!(admin.write_hotp_slot(slot_data, 0).is_ok());
    let slot_data = OtpSlotData::new(2, "test2", HOTP_SECRET, OtpMode::SixDigits);
    assert!(admin.write_hotp_slot(slot_data, 0).is_ok());

    assert!(admin.erase_hotp_slot(1).is_ok());

    let device = admin.device();
    let result = device.get_hotp_slot_name(1);
    assert_eq!(CommandError::SlotNotProgrammed, result.unwrap_err());
    let result = device.get_hotp_code(1);
    assert_eq!(CommandError::SlotNotProgrammed, result.unwrap_err());

    assert_eq!("test2", device.get_hotp_slot_name(2).unwrap());
}

fn configure_totp(admin: &ConfigureOtp, factor: u64) {
    let slot_data = OtpSlotData::new(1, "test-totp", TOTP_SECRET, OtpMode::EightDigits);
    let time_window = 30u64.checked_mul(factor).unwrap();
    assert!(admin.write_totp_slot(slot_data, time_window as u16).is_ok());
}

fn check_totp_codes(device: &GenerateOtp, factor: u64, timestamp_size: TotpTimestampSize) {
    for (i, &(base_time, code)) in TOTP_CODES.iter().enumerate() {
        let time = base_time.checked_mul(factor).unwrap();
        let is_u64 = time > u32::max_value() as u64;
        if is_u64 != (timestamp_size == TotpTimestampSize::U64) {
            continue;
        }

        assert!(device.set_time(time, true).is_ok());
        let result = device.get_totp_code(1);
        assert!(result.is_ok());
        let result_code = result.unwrap();
        assert_eq!(
            code, result_code,
            "TOTP code {} should be {} but is {}",
            i, code, result_code
        );
    }
}

#[test]
#[cfg_attr(not(any(feature = "test-pro", feature = "test-storage")), ignore)]
fn totp_no_pin() {
    // TODO: this test may fail due to bad timing --> find solution
    let admin = get_admin_test_device();
    let config = Config::new(None, None, None, false);
    assert!(admin.write_config(config).is_ok());

    configure_totp(&admin, 1);
    check_totp_codes(admin.deref(), 1, TotpTimestampSize::U32);

    configure_totp(&admin, 2);
    check_totp_codes(admin.deref(), 2, TotpTimestampSize::U32);

    configure_totp(&admin, 1);
    check_totp_codes(&admin.device(), 1, TotpTimestampSize::U32);
}

#[test]
#[cfg_attr(not(any(feature = "test-pro", feature = "test-storage")), ignore)]
#[cfg_attr(feature = "test-storage", should_panic(expected = "assertion failed"))]
// Nitrokey Storage does only support timestamps that fit in a 32-bit unsigned integer.  Therefore
// the last RFC test case is expected to fail.
fn totp_no_pin_64() {
    let admin = get_admin_test_device();
    let config = Config::new(None, None, None, false);
    assert!(admin.write_config(config).is_ok());

    configure_totp(&admin, 1);
    check_totp_codes(admin.deref(), 1, TotpTimestampSize::U64);

    configure_totp(&admin, 2);
    check_totp_codes(admin.deref(), 2, TotpTimestampSize::U64);

    configure_totp(&admin, 1);
    check_totp_codes(&admin.device(), 1, TotpTimestampSize::U64);
}

#[test]
#[cfg_attr(not(any(feature = "test-pro", feature = "test-storage")), ignore)]
fn totp_pin() {
    // TODO: this test may fail due to bad timing --> find solution
    let admin = get_admin_test_device();
    let config = Config::new(None, None, None, true);
    assert!(admin.write_config(config).is_ok());

    configure_totp(&admin, 1);
    let user = admin.device().authenticate_user(USER_PASSWORD).unwrap();
    check_totp_codes(&user, 1, TotpTimestampSize::U32);

    assert!(user.device().get_totp_code(1).is_err());
}

#[test]
#[cfg_attr(not(any(feature = "test-pro", feature = "test-storage")), ignore)]
#[cfg_attr(feature = "test-storage", should_panic(expected = "assertion failed"))]
// See comment for totp_no_pin_64.
fn totp_pin_64() {
    let admin = get_admin_test_device();
    let config = Config::new(None, None, None, true);
    assert!(admin.write_config(config).is_ok());

    configure_totp(&admin, 1);
    let user = admin.device().authenticate_user(USER_PASSWORD).unwrap();
    check_totp_codes(&user, 1, TotpTimestampSize::U64);

    assert!(user.device().get_totp_code(1).is_err());
}

#[test]
#[cfg_attr(not(any(feature = "test-pro", feature = "test-storage")), ignore)]
fn totp_slot_name() {
    let admin = get_admin_test_device();
    let slot_data = OtpSlotData::new(1, "test-totp", TOTP_SECRET, OtpMode::EightDigits);
    assert!(admin.write_totp_slot(slot_data, 0).is_ok());

    let device = admin.device();
    let result = device.get_totp_slot_name(1);
    assert!(result.is_ok());
    assert_eq!("test-totp", result.unwrap());
    let result = device.get_totp_slot_name(16);
    assert_eq!(CommandError::InvalidSlot, result.unwrap_err());
}

#[test]
#[cfg_attr(not(any(feature = "test-pro", feature = "test-storage")), ignore)]
fn totp_error() {
    let admin = get_admin_test_device();
    let slot_data = OtpSlotData::new(1, "", HOTP_SECRET, OtpMode::SixDigits);
    assert_eq!(
        Err(CommandError::NoName),
        admin.write_hotp_slot(slot_data, 0)
    );
    let slot_data = OtpSlotData::new(4, "test", HOTP_SECRET, OtpMode::SixDigits);
    assert_eq!(
        Err(CommandError::InvalidSlot),
        admin.write_hotp_slot(slot_data, 0)
    );
    let code = admin.get_hotp_code(4);
    assert_eq!(CommandError::InvalidSlot, code.unwrap_err());
}

#[test]
#[cfg_attr(not(any(feature = "test-pro", feature = "test-storage")), ignore)]
fn totp_erase() {
    let admin = get_admin_test_device();
    let config = Config::new(None, None, None, false);
    assert!(admin.write_config(config).is_ok());
    let slot_data = OtpSlotData::new(1, "test1", TOTP_SECRET, OtpMode::SixDigits);
    assert!(admin.write_totp_slot(slot_data, 0).is_ok());
    let slot_data = OtpSlotData::new(2, "test2", TOTP_SECRET, OtpMode::SixDigits);
    assert!(admin.write_totp_slot(slot_data, 0).is_ok());

    assert!(admin.erase_totp_slot(1).is_ok());

    let device = admin.device();
    let result = device.get_totp_slot_name(1);
    assert_eq!(CommandError::SlotNotProgrammed, result.unwrap_err());
    let result = device.get_totp_code(1);
    assert_eq!(CommandError::SlotNotProgrammed, result.unwrap_err());

    assert_eq!("test2", device.get_totp_slot_name(2).unwrap());
}
