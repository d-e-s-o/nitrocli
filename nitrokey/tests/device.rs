// Copyright (C) 2018-2020 Robin Krahl <robin.krahl@ireas.org>
// SPDX-License-Identifier: MIT

mod util;

use std::ffi::CStr;
use std::process::Command;
use std::{thread, time};

use nitrokey::{
    Authenticate, CommandError, CommunicationError, Config, ConfigureOtp, Device, DeviceInfo,
    Error, GenerateOtp, GetPasswordSafe, LibraryError, OperationStatus, OtpMode, OtpSlotData,
    Storage, VolumeMode, DEFAULT_ADMIN_PIN, DEFAULT_USER_PIN,
};
use nitrokey_test::test as test_device;

static ADMIN_NEW_PASSWORD: &str = "1234567890";
static UPDATE_PIN: &str = "12345678";
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
fn list_no_devices() {
    let devices = nitrokey::list_devices();
    assert_ok!(Vec::<DeviceInfo>::new(), devices);
}

#[test_device]
fn list_devices(_device: DeviceWrapper) {
    let devices = unwrap_ok!(nitrokey::list_devices());
    for device in devices {
        assert!(!device.path.is_empty());
        if let Some(model) = device.model {
            match model {
                nitrokey::Model::Pro => {
                    assert!(device.serial_number.is_some());
                    let serial_number = device.serial_number.unwrap();
                    assert!(!serial_number.is_empty());
                    assert_valid_serial_number(&serial_number);
                }
                nitrokey::Model::Storage => {
                    assert_eq!(None, device.serial_number);
                }
            }
        }
    }
}

#[test_device]
fn connect_no_device() {
    let mut manager = unwrap_ok!(nitrokey::take());

    assert_cmu_err!(CommunicationError::NotConnected, manager.connect());
    assert_cmu_err!(
        CommunicationError::NotConnected,
        manager.connect_model(nitrokey::Model::Pro)
    );
    assert_cmu_err!(
        CommunicationError::NotConnected,
        manager.connect_model(nitrokey::Model::Storage)
    );
    assert_cmu_err!(CommunicationError::NotConnected, manager.connect_pro());
    assert_cmu_err!(CommunicationError::NotConnected, manager.connect_storage());
}

#[test_device]
fn connect_pro(device: Pro) {
    assert_eq!(device.get_model(), nitrokey::Model::Pro);

    let manager = device.into_manager();
    assert_any_ok!(manager.connect());
    assert_any_ok!(manager.connect_model(nitrokey::Model::Pro));
    assert_any_ok!(manager.connect_pro());
}

#[test_device]
fn connect_storage(device: Storage) {
    assert_eq!(device.get_model(), nitrokey::Model::Storage);

    let manager = device.into_manager();
    assert_any_ok!(manager.connect());
    assert_any_ok!(manager.connect_model(nitrokey::Model::Storage));
    assert_any_ok!(manager.connect_storage());
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
fn connect_path_no_device() {
    let mut manager = unwrap_ok!(nitrokey::take());

    assert_cmu_err!(CommunicationError::NotConnected, manager.connect_path(""));
    assert_cmu_err!(
        CommunicationError::NotConnected,
        manager.connect_path("foobar")
    );
    // TODO: add realistic path
}

#[test_device]
fn connect_path(device: DeviceWrapper) {
    let manager = device.into_manager();

    assert_cmu_err!(CommunicationError::NotConnected, manager.connect_path(""));
    assert_cmu_err!(
        CommunicationError::NotConnected,
        manager.connect_path("foobar")
    );
    // TODO: add realistic path

    let devices = unwrap_ok!(nitrokey::list_devices());
    assert!(!devices.is_empty());
    for device in devices {
        let connected_device = unwrap_ok!(manager.connect_path(device.path));
        assert_eq!(device.model, Some(connected_device.get_model()));
        match device.model.unwrap() {
            nitrokey::Model::Pro => {
                assert!(device.serial_number.is_some());
                assert_ok!(
                    device.serial_number.unwrap(),
                    connected_device.get_serial_number()
                );
            }
            nitrokey::Model::Storage => {
                assert_eq!(None, device.serial_number);
            }
        }
    }
}

#[test_device]
fn disconnect(device: DeviceWrapper) {
    drop(device);
    assert_empty_serial_number();
}

#[test_device]
fn get_status(device: DeviceWrapper) {
    let status = unwrap_ok!(device.get_status());
    assert_ok!(status.firmware_version, device.get_firmware_version());
    let serial_number = format!("{:08x}", status.serial_number);
    assert_ok!(serial_number, device.get_serial_number());
    assert_ok!(status.config, device.get_config());
}

fn assert_valid_serial_number(serial_number: &str) {
    assert!(serial_number.is_ascii());
    assert!(serial_number.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test_device]
fn get_serial_number(device: DeviceWrapper) {
    let serial_number = unwrap_ok!(device.get_serial_number());
    assert_valid_serial_number(&serial_number);
}

#[test_device]
fn get_firmware_version(device: Pro) {
    let version = unwrap_ok!(device.get_firmware_version());
    assert_eq!(0, version.major);
    assert!(version.minor > 0);
}

fn admin_retry<'a, T>(device: T, suffix: &str, count: u8) -> T
where
    T: Authenticate<'a> + Device<'a> + 'a,
{
    let result = device.authenticate_admin(&(DEFAULT_ADMIN_PIN.to_owned() + suffix));
    let device = match result {
        Ok(admin) => admin.device(),
        Err((device, _)) => device,
    };
    assert_ok!(count, device.get_admin_retry_count());
    return device;
}

fn user_retry<'a, T>(device: T, suffix: &str, count: u8) -> T
where
    T: Authenticate<'a> + Device<'a> + 'a,
{
    let result = device.authenticate_user(&(DEFAULT_USER_PIN.to_owned() + suffix));
    let device = match result {
        Ok(admin) => admin.device(),
        Err((device, _)) => device,
    };
    assert_ok!(count, device.get_user_retry_count());
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
    let mut admin = unwrap_ok!(device.authenticate_admin(DEFAULT_ADMIN_PIN));

    let config = Config::new(None, None, None, true);
    assert_ok!((), admin.write_config(config));
    assert_ok!(config, admin.get_config());

    let config = Config::new(None, Some(9), None, true);
    assert_lib_err!(LibraryError::InvalidSlot, admin.write_config(config));

    let config = Config::new(Some(1), None, Some(0), false);
    assert_ok!((), admin.write_config(config));
    assert_ok!(config, admin.get_config());

    let config = Config::new(None, None, None, false);
    assert_ok!((), admin.write_config(config));
    assert_ok!(config, admin.get_config());
}

#[test_device]
fn change_user_pin(device: DeviceWrapper) {
    let device = device.authenticate_user(DEFAULT_USER_PIN).unwrap().device();
    let device = device.authenticate_user(USER_NEW_PASSWORD).unwrap_err().0;

    let mut device = device;
    assert_ok!(
        (),
        device.change_user_pin(DEFAULT_USER_PIN, USER_NEW_PASSWORD)
    );

    let device = device.authenticate_user(DEFAULT_USER_PIN).unwrap_err().0;
    let device = device
        .authenticate_user(USER_NEW_PASSWORD)
        .unwrap()
        .device();

    let mut device = device;
    let result = device.change_user_pin(DEFAULT_USER_PIN, DEFAULT_USER_PIN);
    assert_cmd_err!(CommandError::WrongPassword, result);

    assert_ok!(
        (),
        device.change_user_pin(USER_NEW_PASSWORD, DEFAULT_USER_PIN)
    );

    let device = device.authenticate_user(DEFAULT_USER_PIN).unwrap().device();
    assert!(device.authenticate_user(USER_NEW_PASSWORD).is_err());
}

#[test_device]
fn change_admin_pin(device: DeviceWrapper) {
    let device = device
        .authenticate_admin(DEFAULT_ADMIN_PIN)
        .unwrap()
        .device();
    let mut device = device.authenticate_admin(ADMIN_NEW_PASSWORD).unwrap_err().0;

    assert_ok!(
        (),
        device.change_admin_pin(DEFAULT_ADMIN_PIN, ADMIN_NEW_PASSWORD)
    );

    let device = device.authenticate_admin(DEFAULT_ADMIN_PIN).unwrap_err().0;
    let mut device = device
        .authenticate_admin(ADMIN_NEW_PASSWORD)
        .unwrap()
        .device();

    assert_cmd_err!(
        CommandError::WrongPassword,
        device.change_admin_pin(DEFAULT_ADMIN_PIN, DEFAULT_ADMIN_PIN)
    );

    assert_ok!(
        (),
        device.change_admin_pin(ADMIN_NEW_PASSWORD, DEFAULT_ADMIN_PIN)
    );

    let device = device
        .authenticate_admin(DEFAULT_ADMIN_PIN)
        .unwrap()
        .device();
    device.authenticate_admin(ADMIN_NEW_PASSWORD).unwrap_err();
}

fn require_failed_user_login<'a, D>(device: D, password: &str, error: CommandError) -> D
where
    D: Device<'a> + Authenticate<'a> + 'a,
    nitrokey::User<'a, D>: std::fmt::Debug,
{
    let result = device.authenticate_user(password);
    assert!(result.is_err());
    let err = result.unwrap_err();
    match err.1 {
        Error::CommandError(err) => assert_eq!(error, err),
        _ => assert!(false),
    };
    err.0
}

#[test_device]
fn unlock_user_pin(device: DeviceWrapper) {
    let mut device = device.authenticate_user(DEFAULT_USER_PIN).unwrap().device();
    assert_ok!(
        (),
        device.unlock_user_pin(DEFAULT_ADMIN_PIN, DEFAULT_USER_PIN)
    );
    assert_cmd_err!(
        CommandError::WrongPassword,
        device.unlock_user_pin(DEFAULT_USER_PIN, DEFAULT_USER_PIN)
    );

    // block user PIN
    let wrong_password = DEFAULT_USER_PIN.to_owned() + "foo";
    let device = require_failed_user_login(device, &wrong_password, CommandError::WrongPassword);
    let device = require_failed_user_login(device, &wrong_password, CommandError::WrongPassword);
    let device = require_failed_user_login(device, &wrong_password, CommandError::WrongPassword);
    let mut device =
        require_failed_user_login(device, DEFAULT_USER_PIN, CommandError::WrongPassword);

    // unblock with current PIN
    assert_cmd_err!(
        CommandError::WrongPassword,
        device.unlock_user_pin(DEFAULT_USER_PIN, DEFAULT_USER_PIN)
    );
    assert_ok!(
        (),
        device.unlock_user_pin(DEFAULT_ADMIN_PIN, DEFAULT_USER_PIN)
    );
    let device = device.authenticate_user(DEFAULT_USER_PIN).unwrap().device();

    // block user PIN
    let device = require_failed_user_login(device, &wrong_password, CommandError::WrongPassword);
    let device = require_failed_user_login(device, &wrong_password, CommandError::WrongPassword);
    let device = require_failed_user_login(device, &wrong_password, CommandError::WrongPassword);
    let mut device =
        require_failed_user_login(device, DEFAULT_USER_PIN, CommandError::WrongPassword);

    // unblock with new PIN
    assert_cmd_err!(
        CommandError::WrongPassword,
        device.unlock_user_pin(DEFAULT_USER_PIN, DEFAULT_USER_PIN)
    );
    assert_ok!(
        (),
        device.unlock_user_pin(DEFAULT_ADMIN_PIN, USER_NEW_PASSWORD)
    );

    // reset user PIN
    assert_ok!(
        (),
        device.change_user_pin(USER_NEW_PASSWORD, DEFAULT_USER_PIN)
    );
}

fn assert_utf8_err_or_ne(left: &str, right: Result<String, Error>) {
    match right {
        Ok(s) => assert_ne!(left.to_string(), s),
        Err(Error::Utf8Error(_)) => {}
        Err(err) => panic!("Expected Utf8Error, got {}!", err),
    }
}

#[test_device]
fn factory_reset(device: DeviceWrapper) {
    let mut admin = unwrap_ok!(device.authenticate_admin(DEFAULT_ADMIN_PIN));
    let otp_data = OtpSlotData::new(1, "test", "0123468790", OtpMode::SixDigits);
    assert_ok!((), admin.write_totp_slot(otp_data, 30));

    let mut device = admin.device();
    let mut pws = unwrap_ok!(device.get_password_safe(DEFAULT_USER_PIN));
    assert_ok!((), pws.write_slot(0, "test", "testlogin", "testpw"));
    drop(pws);

    assert_ok!(
        (),
        device.change_user_pin(DEFAULT_USER_PIN, USER_NEW_PASSWORD)
    );
    assert_ok!(
        (),
        device.change_admin_pin(DEFAULT_ADMIN_PIN, ADMIN_NEW_PASSWORD)
    );

    assert_cmd_err!(
        CommandError::WrongPassword,
        device.factory_reset(USER_NEW_PASSWORD)
    );
    assert_cmd_err!(
        CommandError::WrongPassword,
        device.factory_reset(DEFAULT_ADMIN_PIN)
    );
    assert_ok!((), device.factory_reset(ADMIN_NEW_PASSWORD));

    let device = device
        .authenticate_admin(DEFAULT_ADMIN_PIN)
        .unwrap()
        .device();

    let user = unwrap_ok!(device.authenticate_user(DEFAULT_USER_PIN));
    assert_cmd_err!(CommandError::SlotNotProgrammed, user.get_totp_slot_name(1));

    let mut device = user.device();
    let pws = unwrap_ok!(device.get_password_safe(DEFAULT_USER_PIN));
    assert_utf8_err_or_ne("test", pws.get_slot_name(0));
    assert_utf8_err_or_ne("testlogin", pws.get_slot_login(0));
    assert_utf8_err_or_ne("testpw", pws.get_slot_password(0));
    drop(pws);

    assert_ok!(3, device.get_user_retry_count());
    assert_ok!((), device.build_aes_key(DEFAULT_ADMIN_PIN));
}

#[test_device]
fn build_aes_key(device: DeviceWrapper) {
    let mut device = device;
    let mut pws = unwrap_ok!(device.get_password_safe(DEFAULT_USER_PIN));
    assert_ok!((), pws.write_slot(0, "test", "testlogin", "testpw"));
    drop(pws);

    assert_cmd_err!(
        CommandError::WrongPassword,
        device.build_aes_key(DEFAULT_USER_PIN)
    );
    assert_ok!((), device.build_aes_key(DEFAULT_ADMIN_PIN));

    let mut device = device
        .authenticate_admin(DEFAULT_ADMIN_PIN)
        .unwrap()
        .device();

    let pws = unwrap_ok!(device.get_password_safe(DEFAULT_USER_PIN));
    assert_utf8_err_or_ne("test", pws.get_slot_name(0));
    assert_utf8_err_or_ne("testlogin", pws.get_slot_login(0));
    assert_utf8_err_or_ne("testpw", pws.get_slot_password(0));
}

#[test_device]
fn change_update_pin(device: Storage) {
    let mut device = device;
    assert_cmd_err!(
        CommandError::WrongPassword,
        device.change_update_pin(UPDATE_NEW_PIN, UPDATE_PIN)
    );
    assert_ok!((), device.change_update_pin(UPDATE_PIN, UPDATE_NEW_PIN));
    assert_ok!((), device.change_update_pin(UPDATE_NEW_PIN, UPDATE_PIN));
}

#[test_device]
fn encrypted_volume(device: Storage) {
    let mut device = device;
    assert_ok!((), device.lock());

    assert_eq!(1, count_nitrokey_block_devices());
    assert_ok!((), device.disable_encrypted_volume());
    assert_eq!(1, count_nitrokey_block_devices());
    assert_cmd_err!(
        CommandError::WrongPassword,
        device.enable_encrypted_volume("123")
    );
    assert_eq!(1, count_nitrokey_block_devices());
    assert_ok!((), device.enable_encrypted_volume(DEFAULT_USER_PIN));
    assert_eq!(2, count_nitrokey_block_devices());
    assert_ok!((), device.disable_encrypted_volume());
    assert_eq!(1, count_nitrokey_block_devices());
}

#[test_device]
fn hidden_volume(device: Storage) {
    let mut device = device;
    assert_ok!((), device.lock());

    assert_eq!(1, count_nitrokey_block_devices());
    assert_ok!((), device.disable_hidden_volume());
    assert_eq!(1, count_nitrokey_block_devices());

    assert_ok!((), device.enable_encrypted_volume(DEFAULT_USER_PIN));
    assert_eq!(2, count_nitrokey_block_devices());

    // TODO: why this error code?
    assert_cmd_err!(
        CommandError::WrongPassword,
        device.create_hidden_volume(5, 0, 100, "hiddenpw")
    );
    assert_ok!((), device.create_hidden_volume(0, 20, 21, "hidden-pw"));
    assert_ok!((), device.create_hidden_volume(0, 20, 21, "hiddenpassword"));
    assert_ok!((), device.create_hidden_volume(1, 0, 1, "otherpw"));
    // TODO: test invalid range (not handled by libnitrokey)
    assert_eq!(2, count_nitrokey_block_devices());

    assert_cmd_err!(
        CommandError::WrongPassword,
        device.enable_hidden_volume("blubb")
    );
    assert_ok!((), device.enable_hidden_volume("hiddenpassword"));
    assert_eq!(2, count_nitrokey_block_devices());
    assert_ok!((), device.enable_hidden_volume("otherpw"));
    assert_eq!(2, count_nitrokey_block_devices());

    assert_ok!((), device.disable_hidden_volume());
    assert_eq!(1, count_nitrokey_block_devices());
}

#[test_device]
fn lock(device: Storage) {
    let mut device = device;
    assert_ok!((), device.enable_encrypted_volume(DEFAULT_USER_PIN));
    assert_ok!((), device.lock());
    assert_eq!(1, count_nitrokey_block_devices());
}

#[test_device]
fn set_encrypted_volume_mode(device: Storage) {
    // This test case does not check the device status as the command only works with firmware
    // version 0.49.  For later versions, it does not do anything and always returns Ok(()).
    let mut device = device;

    assert_ok!(
        (),
        device.set_encrypted_volume_mode(DEFAULT_ADMIN_PIN, VolumeMode::ReadOnly)
    );

    // TODO: re-enable once the password is checked in the firmware
    // assert_cmd_err!(
    //     CommandError::WrongPassword,
    //     device.set_encrypted_volume_mode(DEFAULT_USER_PIN, VolumeMode::ReadOnly)
    // );

    assert_ok!(
        (),
        device.set_encrypted_volume_mode(DEFAULT_ADMIN_PIN, VolumeMode::ReadOnly)
    );
    assert_ok!(
        (),
        device.set_encrypted_volume_mode(DEFAULT_ADMIN_PIN, VolumeMode::ReadWrite)
    );
    assert_ok!(
        (),
        device.set_encrypted_volume_mode(DEFAULT_ADMIN_PIN, VolumeMode::ReadOnly)
    );
}

#[test_device]
fn set_unencrypted_volume_mode(device: Storage) {
    fn assert_mode(device: &Storage, mode: VolumeMode) {
        let status = unwrap_ok!(device.get_storage_status());
        assert_eq!(
            status.unencrypted_volume.read_only,
            mode == VolumeMode::ReadOnly
        );
    }

    fn assert_success(device: &mut Storage, mode: VolumeMode) {
        assert_ok!(
            (),
            device.set_unencrypted_volume_mode(DEFAULT_ADMIN_PIN, mode)
        );
        assert_mode(&device, mode);
    }

    let mut device = device;
    assert_success(&mut device, VolumeMode::ReadOnly);

    assert_cmd_err!(
        CommandError::WrongPassword,
        device.set_unencrypted_volume_mode(DEFAULT_USER_PIN, VolumeMode::ReadOnly)
    );
    assert_mode(&device, VolumeMode::ReadOnly);

    assert_success(&mut device, VolumeMode::ReadWrite);
    assert_success(&mut device, VolumeMode::ReadWrite);
    assert_success(&mut device, VolumeMode::ReadOnly);
}

#[test_device]
fn get_storage_status(device: Storage) {
    let status = unwrap_ok!(device.get_storage_status());
    assert!(status.serial_number_sd_card > 0);
    assert!(status.serial_number_smart_card > 0);
}

#[test_device]
fn get_production_info(device: Storage) {
    let info = unwrap_ok!(device.get_production_info());
    assert_eq!(0, info.firmware_version.major);
    assert!(info.firmware_version.minor != 0);
    assert!(info.serial_number_cpu != 0);
    assert!(info.sd_card.serial_number != 0);
    assert!(info.sd_card.size > 0);
    assert!(info.sd_card.manufacturing_year > 10);
    assert!(info.sd_card.manufacturing_year < 100);
    // TODO: month value is not valid atm
    // assert!(info.sd_card.manufacturing_month < 12);
    assert!(info.sd_card.oem != 0);
    assert!(info.sd_card.manufacturer != 0);

    let status = unwrap_ok!(device.get_storage_status());
    assert_eq!(status.firmware_version, info.firmware_version);
    assert_eq!(status.serial_number_sd_card, info.sd_card.serial_number);
}

#[test_device]
fn clear_new_sd_card_warning(device: Storage) {
    let mut device = device;
    assert_ok!((), device.factory_reset(DEFAULT_ADMIN_PIN));
    thread::sleep(time::Duration::from_secs(3));
    assert_ok!((), device.build_aes_key(DEFAULT_ADMIN_PIN));

    // We have to perform an SD card operation to reset the new_sd_card_found field
    assert_ok!((), device.lock());

    let status = unwrap_ok!(device.get_storage_status());
    assert!(status.new_sd_card_found);

    assert_ok!((), device.clear_new_sd_card_warning(DEFAULT_ADMIN_PIN));

    let status = unwrap_ok!(device.get_storage_status());
    assert!(!status.new_sd_card_found);
}

#[test_device]
fn get_sd_card_usage(device: Storage) {
    let range = unwrap_ok!(device.get_sd_card_usage());

    assert!(range.end >= range.start);
    assert!(range.end <= 100);
}

#[test_device]
fn get_operation_status(device: Storage) {
    assert_ok!(OperationStatus::Idle, device.get_operation_status());
}

#[test_device]
#[ignore]
fn fill_sd_card(device: Storage) {
    // This test takes up to 60 min to execute and is therefore ignored by default.  Use `cargo
    // test -- --ignored fill_sd_card` to run the test.

    let mut device = device;
    assert_ok!((), device.factory_reset(DEFAULT_ADMIN_PIN));
    thread::sleep(time::Duration::from_secs(3));
    assert_ok!((), device.build_aes_key(DEFAULT_ADMIN_PIN));

    let status = unwrap_ok!(device.get_storage_status());
    assert!(!status.filled_with_random);

    assert_ok!((), device.fill_sd_card(DEFAULT_ADMIN_PIN));
    assert_cmd_err!(CommandError::WrongCrc, device.get_status());

    let mut status = OperationStatus::Ongoing(0);
    let mut last_progress = 0u8;
    while status != OperationStatus::Idle {
        status = unwrap_ok!(device.get_operation_status());
        if let OperationStatus::Ongoing(progress) = status {
            assert!(progress <= 100, "progress = {}", progress);
            assert!(
                progress >= last_progress,
                "progress = {}, last_progress = {}",
                progress,
                last_progress
            );
            last_progress = progress;

            thread::sleep(time::Duration::from_secs(10));
        }
    }

    let status = unwrap_ok!(device.get_storage_status());
    assert!(status.filled_with_random);
}

#[test_device]
fn export_firmware(device: Storage) {
    let mut device = device;
    assert_cmd_err!(
        CommandError::WrongPassword,
        device.export_firmware("someadminpn")
    );
    assert_ok!((), device.export_firmware(DEFAULT_ADMIN_PIN));
    assert_ok!(
        (),
        device.set_unencrypted_volume_mode(DEFAULT_ADMIN_PIN, VolumeMode::ReadWrite)
    );
    assert_ok!((), device.export_firmware(DEFAULT_ADMIN_PIN));
    assert_ok!(
        (),
        device.set_unencrypted_volume_mode(DEFAULT_ADMIN_PIN, VolumeMode::ReadOnly)
    );
}
