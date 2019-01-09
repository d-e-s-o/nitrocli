mod util;

use std::ffi::CStr;
use std::process::Command;
use std::{thread, time};

use nitrokey::{
    Authenticate, CommandError, Config, ConfigureOtp, Device, GenerateOtp, GetPasswordSafe,
    OtpMode, OtpSlotData,
};
use nitrokey_test::test as test_device;

use crate::util::{ADMIN_PASSWORD, UPDATE_PIN, USER_PASSWORD};

static ADMIN_NEW_PASSWORD: &str = "1234567890";
static UPDATE_NEW_PIN: &str = "87654321";
static USER_NEW_PASSWORD: &str = "abcdefghij";

fn count_nitrokey_block_devices() -> usize {
    thread::sleep(time::Duration::from_secs(2));
    let output = Command::new("lsblk")
        .args(&["-o", "MODEL"])
        .output()
        .expect("Could not list block devices");
    String::from_utf8_lossy(&output.stdout)
        .split("\n")
        .filter(|&s| s.replace("_", " ") == "Nitrokey Storage")
        .count()
}

#[test_device]
fn connect_no_device() {
    assert!(nitrokey::connect().is_err());
    assert!(nitrokey::connect_model(nitrokey::Model::Pro).is_err());
    assert!(nitrokey::connect_model(nitrokey::Model::Storage).is_err());
    assert!(nitrokey::Pro::connect().is_err());
    assert!(nitrokey::Storage::connect().is_err());
}

#[test_device]
fn connect_pro(device: Pro) {
    assert_eq!(device.get_model(), nitrokey::Model::Pro);
    drop(device);

    assert!(nitrokey::connect().is_ok());
    assert!(nitrokey::connect_model(nitrokey::Model::Pro).is_ok());
    assert!(nitrokey::Pro::connect().is_ok());

    assert!(nitrokey::connect_model(nitrokey::Model::Storage).is_err());
    assert!(nitrokey::Storage::connect().is_err());
}

#[test_device]
fn connect_storage(device: Storage) {
    assert_eq!(device.get_model(), nitrokey::Model::Storage);
    drop(device);

    assert!(nitrokey::connect().is_ok());
    assert!(nitrokey::connect_model(nitrokey::Model::Storage).is_ok());
    assert!(nitrokey::Storage::connect().is_ok());

    assert!(nitrokey::connect_model(nitrokey::Model::Pro).is_err());
    assert!(nitrokey::Pro::connect().is_err());
}

fn assert_empty_serial_number() {
    unsafe {
        let ptr = nitrokey_sys::NK_device_serial_number();
        assert!(!ptr.is_null());
        let cstr = CStr::from_ptr(ptr);
        assert_eq!(cstr.to_string_lossy(), "");
    }
}

#[test_device]
fn disconnect(device: DeviceWrapper) {
    drop(device);
    assert_empty_serial_number();
}

#[test_device]
fn get_serial_number(device: DeviceWrapper) {
    let result = device.get_serial_number();
    assert!(result.is_ok());
    let serial_number = result.unwrap();
    assert!(serial_number.is_ascii());
    assert!(serial_number.chars().all(|c| c.is_ascii_hexdigit()));
}
#[test_device]
fn get_firmware_version(device: Pro) {
    assert_eq!(0, device.get_major_firmware_version());
    let minor = device.get_minor_firmware_version();
    assert!(minor > 0);
}

fn admin_retry<T: Authenticate + Device>(device: T, suffix: &str, count: u8) -> T {
    let result = device.authenticate_admin(&(ADMIN_PASSWORD.to_owned() + suffix));
    let device = match result {
        Ok(admin) => admin.device(),
        Err((device, _)) => device,
    };
    assert_eq!(count, device.get_admin_retry_count());
    return device;
}

fn user_retry<T: Authenticate + Device>(device: T, suffix: &str, count: u8) -> T {
    let result = device.authenticate_user(&(USER_PASSWORD.to_owned() + suffix));
    let device = match result {
        Ok(admin) => admin.device(),
        Err((device, _)) => device,
    };
    assert_eq!(count, device.get_user_retry_count());
    return device;
}

#[test_device]
fn get_retry_count(device: DeviceWrapper) {
    let device = admin_retry(device, "", 3);
    let device = admin_retry(device, "123", 2);
    let device = admin_retry(device, "456", 1);
    let device = admin_retry(device, "", 3);

    let device = user_retry(device, "", 3);
    let device = user_retry(device, "123", 2);
    let device = user_retry(device, "456", 1);
    user_retry(device, "", 3);
}

#[test_device]
fn config(device: DeviceWrapper) {
    let admin = device.authenticate_admin(ADMIN_PASSWORD).unwrap();
    let config = Config::new(None, None, None, true);
    assert_eq!(Ok(()), admin.write_config(config));
    let get_config = admin.get_config().unwrap();
    assert_eq!(config, get_config);

    let config = Config::new(None, Some(9), None, true);
    assert_eq!(Err(CommandError::InvalidSlot), admin.write_config(config));

    let config = Config::new(Some(1), None, Some(0), false);
    assert_eq!(Ok(()), admin.write_config(config));
    let get_config = admin.get_config().unwrap();
    assert_eq!(config, get_config);

    let config = Config::new(None, None, None, false);
    assert_eq!(Ok(()), admin.write_config(config));
    let get_config = admin.get_config().unwrap();
    assert_eq!(config, get_config);
}

#[test_device]
fn change_user_pin(device: DeviceWrapper) {
    let device = device.authenticate_user(USER_PASSWORD).unwrap().device();
    let device = device.authenticate_user(USER_NEW_PASSWORD).unwrap_err().0;

    assert!(device
        .change_user_pin(USER_PASSWORD, USER_NEW_PASSWORD)
        .is_ok());

    let device = device.authenticate_user(USER_PASSWORD).unwrap_err().0;
    let device = device
        .authenticate_user(USER_NEW_PASSWORD)
        .unwrap()
        .device();

    let result = device.change_user_pin(USER_PASSWORD, USER_PASSWORD);
    assert_eq!(Err(CommandError::WrongPassword), result);

    assert!(device
        .change_user_pin(USER_NEW_PASSWORD, USER_PASSWORD)
        .is_ok());

    let device = device.authenticate_user(USER_PASSWORD).unwrap().device();
    assert!(device.authenticate_user(USER_NEW_PASSWORD).is_err());
}

#[test_device]
fn change_admin_pin(device: DeviceWrapper) {
    let device = device.authenticate_admin(ADMIN_PASSWORD).unwrap().device();
    let device = device.authenticate_admin(ADMIN_NEW_PASSWORD).unwrap_err().0;

    assert!(device
        .change_admin_pin(ADMIN_PASSWORD, ADMIN_NEW_PASSWORD)
        .is_ok());

    let device = device.authenticate_admin(ADMIN_PASSWORD).unwrap_err().0;
    let device = device
        .authenticate_admin(ADMIN_NEW_PASSWORD)
        .unwrap()
        .device();

    assert_eq!(
        Err(CommandError::WrongPassword),
        device.change_admin_pin(ADMIN_PASSWORD, ADMIN_PASSWORD)
    );

    assert!(device
        .change_admin_pin(ADMIN_NEW_PASSWORD, ADMIN_PASSWORD)
        .is_ok());

    let device = device.authenticate_admin(ADMIN_PASSWORD).unwrap().device();
    device.authenticate_admin(ADMIN_NEW_PASSWORD).unwrap_err();
}

fn require_failed_user_login<D>(device: D, password: &str, error: CommandError) -> D
where
    D: Device + Authenticate,
    nitrokey::User<D>: std::fmt::Debug,
{
    let result = device.authenticate_user(password);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(error, err.1);
    err.0
}

#[test_device]
fn unlock_user_pin(device: DeviceWrapper) {
    let device = device.authenticate_user(USER_PASSWORD).unwrap().device();
    assert!(device
        .unlock_user_pin(ADMIN_PASSWORD, USER_PASSWORD)
        .is_ok());
    assert_eq!(
        Err(CommandError::WrongPassword),
        device.unlock_user_pin(USER_PASSWORD, USER_PASSWORD)
    );

    // block user PIN
    let wrong_password = USER_PASSWORD.to_owned() + "foo";
    let device = require_failed_user_login(device, &wrong_password, CommandError::WrongPassword);
    let device = require_failed_user_login(device, &wrong_password, CommandError::WrongPassword);
    let device = require_failed_user_login(device, &wrong_password, CommandError::WrongPassword);
    let device = require_failed_user_login(device, USER_PASSWORD, CommandError::WrongPassword);

    // unblock with current PIN
    assert_eq!(
        Err(CommandError::WrongPassword),
        device.unlock_user_pin(USER_PASSWORD, USER_PASSWORD)
    );
    assert!(device
        .unlock_user_pin(ADMIN_PASSWORD, USER_PASSWORD)
        .is_ok());
    let device = device.authenticate_user(USER_PASSWORD).unwrap().device();

    // block user PIN
    let device = require_failed_user_login(device, &wrong_password, CommandError::WrongPassword);
    let device = require_failed_user_login(device, &wrong_password, CommandError::WrongPassword);
    let device = require_failed_user_login(device, &wrong_password, CommandError::WrongPassword);
    let device = require_failed_user_login(device, USER_PASSWORD, CommandError::WrongPassword);

    // unblock with new PIN
    assert_eq!(
        Err(CommandError::WrongPassword),
        device.unlock_user_pin(USER_PASSWORD, USER_PASSWORD)
    );
    assert!(device
        .unlock_user_pin(ADMIN_PASSWORD, USER_NEW_PASSWORD)
        .is_ok());

    // reset user PIN
    assert!(device
        .change_user_pin(USER_NEW_PASSWORD, USER_PASSWORD)
        .is_ok());
}

#[test_device]
fn factory_reset(device: DeviceWrapper) {
    assert_eq!(
        Ok(()),
        device.change_user_pin(USER_PASSWORD, USER_NEW_PASSWORD)
    );
    assert_eq!(
        Ok(()),
        device.change_admin_pin(ADMIN_PASSWORD, ADMIN_NEW_PASSWORD)
    );

    let admin = device.authenticate_admin(ADMIN_NEW_PASSWORD).unwrap();
    let otp_data = OtpSlotData::new(1, "test", "0123468790", OtpMode::SixDigits);
    assert_eq!(Ok(()), admin.write_totp_slot(otp_data, 30));

    let device = admin.device();
    let pws = device.get_password_safe(USER_NEW_PASSWORD).unwrap();
    assert_eq!(Ok(()), pws.write_slot(0, "test", "testlogin", "testpw"));
    drop(pws);

    assert_eq!(
        Err(CommandError::WrongPassword),
        device.factory_reset(USER_NEW_PASSWORD)
    );
    assert_eq!(
        Err(CommandError::WrongPassword),
        device.factory_reset(ADMIN_PASSWORD)
    );
    assert_eq!(Ok(()), device.factory_reset(ADMIN_NEW_PASSWORD));

    let device = device.authenticate_admin(ADMIN_PASSWORD).unwrap().device();

    let user = device.authenticate_user(USER_PASSWORD).unwrap();
    assert_eq!(
        Err(CommandError::SlotNotProgrammed),
        user.get_totp_slot_name(1)
    );

    let device = user.device();
    let pws = device.get_password_safe(USER_PASSWORD).unwrap();
    assert_ne!("test".to_string(), pws.get_slot_name(0).unwrap());
    assert_ne!("testlogin".to_string(), pws.get_slot_login(0).unwrap());
    assert_ne!("testpw".to_string(), pws.get_slot_password(0).unwrap());

    assert_eq!(Ok(()), device.build_aes_key(ADMIN_PASSWORD));
}

#[test_device]
fn build_aes_key(device: DeviceWrapper) {
    let pws = device.get_password_safe(USER_PASSWORD).unwrap();
    assert_eq!(Ok(()), pws.write_slot(0, "test", "testlogin", "testpw"));
    drop(pws);

    assert_eq!(
        Err(CommandError::WrongPassword),
        device.build_aes_key(USER_PASSWORD)
    );
    assert_eq!(Ok(()), device.build_aes_key(ADMIN_PASSWORD));

    let device = device.authenticate_admin(ADMIN_PASSWORD).unwrap().device();

    let pws = device.get_password_safe(USER_PASSWORD).unwrap();
    assert_ne!("test".to_string(), pws.get_slot_name(0).unwrap());
    assert_ne!("testlogin".to_string(), pws.get_slot_login(0).unwrap());
    assert_ne!("testpw".to_string(), pws.get_slot_password(0).unwrap());
}

#[test_device]
fn change_update_pin(device: Storage) {
    assert_eq!(
        Err(CommandError::WrongPassword),
        device.change_update_pin(UPDATE_NEW_PIN, UPDATE_PIN)
    );
    assert_eq!(Ok(()), device.change_update_pin(UPDATE_PIN, UPDATE_NEW_PIN));
    assert_eq!(Ok(()), device.change_update_pin(UPDATE_NEW_PIN, UPDATE_PIN));
}

#[test_device]
fn encrypted_volume(device: Storage) {
    assert_eq!(Ok(()), device.lock());

    assert_eq!(1, count_nitrokey_block_devices());
    assert_eq!(Ok(()), device.disable_encrypted_volume());
    assert_eq!(1, count_nitrokey_block_devices());
    assert_eq!(
        Err(CommandError::WrongPassword),
        device.enable_encrypted_volume("123")
    );
    assert_eq!(1, count_nitrokey_block_devices());
    assert_eq!(Ok(()), device.enable_encrypted_volume(USER_PASSWORD));
    assert_eq!(2, count_nitrokey_block_devices());
    assert_eq!(Ok(()), device.disable_encrypted_volume());
    assert_eq!(1, count_nitrokey_block_devices());
}

#[test_device]
fn hidden_volume(device: Storage) {
    assert_eq!(Ok(()), device.lock());

    assert_eq!(1, count_nitrokey_block_devices());
    assert_eq!(Ok(()), device.disable_hidden_volume());
    assert_eq!(1, count_nitrokey_block_devices());

    assert_eq!(Ok(()), device.enable_encrypted_volume(USER_PASSWORD));
    assert_eq!(2, count_nitrokey_block_devices());

    // TODO: why this error code?
    assert_eq!(
        Err(CommandError::WrongPassword),
        device.create_hidden_volume(5, 0, 100, "hiddenpw")
    );
    assert_eq!(Ok(()), device.create_hidden_volume(0, 20, 21, "hidden-pw"));
    assert_eq!(
        Ok(()),
        device.create_hidden_volume(0, 20, 21, "hiddenpassword")
    );
    assert_eq!(Ok(()), device.create_hidden_volume(1, 0, 1, "otherpw"));
    // TODO: test invalid range (not handled by libnitrokey)
    assert_eq!(2, count_nitrokey_block_devices());

    assert_eq!(
        Err(CommandError::WrongPassword),
        device.enable_hidden_volume("blubb")
    );
    assert_eq!(Ok(()), device.enable_hidden_volume("hiddenpassword"));
    assert_eq!(2, count_nitrokey_block_devices());
    assert_eq!(Ok(()), device.enable_hidden_volume("otherpw"));
    assert_eq!(2, count_nitrokey_block_devices());

    assert_eq!(Ok(()), device.disable_hidden_volume());
    assert_eq!(1, count_nitrokey_block_devices());
}

#[test_device]
fn lock(device: Storage) {
    assert_eq!(Ok(()), device.enable_encrypted_volume(USER_PASSWORD));
    assert_eq!(Ok(()), device.lock());
    assert_eq!(1, count_nitrokey_block_devices());
}

#[test_device]
fn get_storage_status(device: Storage) {
    let status = device.get_status().unwrap();

    assert!(status.serial_number_sd_card > 0);
    assert!(status.serial_number_smart_card > 0);
}
