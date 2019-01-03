mod util;

use std::ffi::CStr;
use std::process::Command;
use std::{thread, time};

use nitrokey::{Authenticate, CommandError, Config, Device, Storage};

use crate::util::{Target, ADMIN_PASSWORD, USER_PASSWORD};

static ADMIN_NEW_PASSWORD: &str = "1234567890";
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

#[test]
#[cfg_attr(any(feature = "test-pro", feature = "test-storage"), ignore)]
fn connect_no_device() {
    assert!(nitrokey::connect().is_err());
    assert!(nitrokey::Pro::connect().is_err());
    assert!(nitrokey::Storage::connect().is_err());
}

#[test]
#[cfg_attr(not(feature = "test-pro"), ignore)]
fn connect_pro() {
    assert!(nitrokey::connect().is_ok());
    assert!(nitrokey::Pro::connect().is_ok());
    assert!(nitrokey::Storage::connect().is_err());
    match nitrokey::connect().unwrap() {
        nitrokey::DeviceWrapper::Pro(_) => assert!(true),
        nitrokey::DeviceWrapper::Storage(_) => assert!(false),
    };
}

#[test]
#[cfg_attr(not(feature = "test-storage"), ignore)]
fn connect_storage() {
    assert!(nitrokey::connect().is_ok());
    assert!(nitrokey::Pro::connect().is_err());
    assert!(nitrokey::Storage::connect().is_ok());
    match nitrokey::connect().unwrap() {
        nitrokey::DeviceWrapper::Pro(_) => assert!(false),
        nitrokey::DeviceWrapper::Storage(_) => assert!(true),
    };
}

fn assert_empty_serial_number() {
    unsafe {
        let ptr = nitrokey_sys::NK_device_serial_number();
        assert!(!ptr.is_null());
        let cstr = CStr::from_ptr(ptr);
        assert_eq!(cstr.to_string_lossy(), "");
    }
}

#[test]
#[cfg_attr(not(any(feature = "test-pro", feature = "test-storage")), ignore)]
fn disconnect() {
    Target::connect().unwrap();
    assert_empty_serial_number();
    Target::connect()
        .unwrap()
        .authenticate_admin(ADMIN_PASSWORD)
        .unwrap();
    assert_empty_serial_number();
    Target::connect()
        .unwrap()
        .authenticate_user(USER_PASSWORD)
        .unwrap();
    assert_empty_serial_number();
}

fn require_model(model: nitrokey::Model) {
    assert_eq!(model, nitrokey::connect().unwrap().get_model());
    assert_eq!(model, Target::connect().unwrap().get_model());
}

#[test]
#[cfg_attr(not(feature = "test-pro"), ignore)]
fn get_model_pro() {
    require_model(nitrokey::Model::Pro);
}

#[test]
#[cfg_attr(not(feature = "test-storage"), ignore)]
fn get_model_storage() {
    require_model(nitrokey::Model::Storage);
}

#[test]
#[cfg_attr(not(any(feature = "test-pro", feature = "test-storage")), ignore)]
fn get_serial_number() {
    let device = Target::connect().unwrap();
    let result = device.get_serial_number();
    assert!(result.is_ok());
    let serial_number = result.unwrap();
    assert!(serial_number.is_ascii());
    assert!(serial_number.chars().all(|c| c.is_ascii_hexdigit()));
}
#[test]
#[cfg_attr(not(feature = "test-pro"), ignore)]
fn get_firmware_version() {
    let device = Target::connect().unwrap();
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

#[test]
#[cfg_attr(not(any(feature = "test-pro", feature = "test-storage")), ignore)]
fn get_retry_count() {
    let device = Target::connect().unwrap();

    let device = admin_retry(device, "", 3);
    let device = admin_retry(device, "123", 2);
    let device = admin_retry(device, "456", 1);
    let device = admin_retry(device, "", 3);

    let device = user_retry(device, "", 3);
    let device = user_retry(device, "123", 2);
    let device = user_retry(device, "456", 1);
    user_retry(device, "", 3);
}

#[test]
#[cfg_attr(not(any(feature = "test-pro", feature = "test-storage")), ignore)]
fn config() {
    let device = Target::connect().unwrap();
    let admin = device.authenticate_admin(ADMIN_PASSWORD).unwrap();
    let config = Config::new(None, None, None, true);
    assert!(admin.write_config(config).is_ok());
    let get_config = admin.get_config().unwrap();
    assert_eq!(config, get_config);

    let config = Config::new(None, Some(9), None, true);
    assert_eq!(Err(CommandError::InvalidSlot), admin.write_config(config));

    let config = Config::new(Some(1), None, Some(0), false);
    assert!(admin.write_config(config).is_ok());
    let get_config = admin.get_config().unwrap();
    assert_eq!(config, get_config);

    let config = Config::new(None, None, None, false);
    assert!(admin.write_config(config).is_ok());
    let get_config = admin.get_config().unwrap();
    assert_eq!(config, get_config);
}

#[test]
#[cfg_attr(not(any(feature = "test-pro", feature = "test-storage")), ignore)]
fn change_user_pin() {
    let device = Target::connect().unwrap();
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

#[test]
#[cfg_attr(not(any(feature = "test-pro", feature = "test-storage")), ignore)]
fn change_admin_pin() {
    let device = Target::connect().unwrap();
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

fn require_failed_user_login(device: Target, password: &str, error: CommandError) -> Target {
    let result = device.authenticate_user(password);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(error, err.1);
    err.0
}

#[test]
#[cfg_attr(not(any(feature = "test-pro", feature = "test-storage")), ignore)]
fn unlock_user_pin() {
    let device = Target::connect().unwrap();
    let device = device.authenticate_user(USER_PASSWORD).unwrap().device();
    assert!(device
        .unlock_user_pin(ADMIN_PASSWORD, USER_PASSWORD)
        .is_ok());
    assert_eq!(
        Err(CommandError::WrongPassword),
        device.unlock_user_pin(USER_PASSWORD, USER_PASSWORD)
    );

    let wrong_password = USER_PASSWORD.to_owned() + "foo";
    let device = require_failed_user_login(device, &wrong_password, CommandError::WrongPassword);
    let device = require_failed_user_login(device, &wrong_password, CommandError::WrongPassword);
    let device = require_failed_user_login(device, &wrong_password, CommandError::WrongPassword);
    let device = require_failed_user_login(device, USER_PASSWORD, CommandError::WrongPassword);

    assert_eq!(
        Err(CommandError::WrongPassword),
        device.unlock_user_pin(USER_PASSWORD, USER_PASSWORD)
    );
    assert!(device
        .unlock_user_pin(ADMIN_PASSWORD, USER_PASSWORD)
        .is_ok());
    device.authenticate_user(USER_PASSWORD).unwrap();
}

#[test]
#[cfg_attr(not(feature = "test-storage"), ignore)]
fn encrypted_volume() {
    let device = Storage::connect().unwrap();
    assert!(device.lock().is_ok());

    assert_eq!(1, count_nitrokey_block_devices());
    assert!(device.disable_encrypted_volume().is_ok());
    assert_eq!(1, count_nitrokey_block_devices());
    assert_eq!(
        Err(CommandError::WrongPassword),
        device.enable_encrypted_volume("123")
    );
    assert_eq!(1, count_nitrokey_block_devices());
    assert!(device.enable_encrypted_volume(USER_PASSWORD).is_ok());
    assert_eq!(2, count_nitrokey_block_devices());
    assert!(device.disable_encrypted_volume().is_ok());
    assert_eq!(1, count_nitrokey_block_devices());
}

#[test]
#[cfg_attr(not(feature = "test-storage"), ignore)]
fn lock() {
    let device = Storage::connect().unwrap();

    assert!(device.enable_encrypted_volume(USER_PASSWORD).is_ok());
    assert!(device.lock().is_ok());
    assert_eq!(1, count_nitrokey_block_devices());
}

#[test]
#[cfg_attr(not(feature = "test-storage"), ignore)]
fn get_storage_status() {
    let device = Storage::connect().unwrap();
    let status = device.get_status().unwrap();

    assert!(status.serial_number_sd_card > 0);
    assert!(status.serial_number_smart_card > 0);
}
