// Copyright (C) 2018-2019 Robin Krahl <robin.krahl@ireas.org>
// SPDX-License-Identifier: MIT

mod util;

use std::ffi::CStr;

use libc::{c_int, c_void, free};
use nitrokey::{
    CommandError, Device, Error, GetPasswordSafe, LibraryError, PasswordSafe, DEFAULT_ADMIN_PIN,
    DEFAULT_USER_PIN, SLOT_COUNT,
};
use nitrokey_sys;
use nitrokey_test::test as test_device;

fn get_slot_name_direct(slot: u8) -> Result<String, Error> {
    let ptr = unsafe { nitrokey_sys::NK_get_password_safe_slot_name(slot) };
    if ptr.is_null() {
        return Err(Error::UnexpectedError);
    }
    let s = unsafe { CStr::from_ptr(ptr).to_string_lossy().into_owned() };
    unsafe { free(ptr as *mut c_void) };
    match s.is_empty() {
        true => {
            let error = unsafe { nitrokey_sys::NK_get_last_command_status() } as c_int;
            match error {
                0 => Ok(s),
                other => Err(Error::from(other)),
            }
        }
        false => Ok(s),
    }
}

fn get_pws<'a, T>(device: &mut T) -> PasswordSafe<'_, 'a>
where
    T: Device<'a>,
{
    unwrap_ok!(device.get_password_safe(DEFAULT_USER_PIN))
}

#[test_device]
fn enable(device: DeviceWrapper) {
    let mut device = device;
    assert_cmd_err!(
        CommandError::WrongPassword,
        device.get_password_safe(&(DEFAULT_USER_PIN.to_owned() + "123"))
    );
    assert_any_ok!(device.get_password_safe(DEFAULT_USER_PIN));
    assert_cmd_err!(
        CommandError::WrongPassword,
        device.get_password_safe(DEFAULT_ADMIN_PIN)
    );
    assert_any_ok!(device.get_password_safe(DEFAULT_USER_PIN));
}

#[test_device]
fn drop(device: DeviceWrapper) {
    let mut device = device;
    {
        let mut pws = get_pws(&mut device);
        assert_ok!((), pws.write_slot(1, "name", "login", "password"));
        assert_ok!("name".to_string(), pws.get_slot_name(1));
        let result = get_slot_name_direct(1);
        assert_ok!(String::from("name"), result);
    }
    let result = get_slot_name_direct(1);
    assert_ok!(String::from("name"), result);
    assert_ok!((), device.lock());
    let result = get_slot_name_direct(1);
    assert_cmd_err!(CommandError::NotAuthorized, result);
}

#[test_device]
fn get_status(device: DeviceWrapper) {
    let mut device = device;
    let mut pws = get_pws(&mut device);
    for i in 0..SLOT_COUNT {
        assert_ok!((), pws.erase_slot(i));
    }
    let status = unwrap_ok!(pws.get_slot_status());
    assert_eq!(status, [false; SLOT_COUNT as usize]);

    assert_ok!((), pws.write_slot(1, "name", "login", "password"));
    let status = unwrap_ok!(pws.get_slot_status());
    for i in 0..SLOT_COUNT {
        assert_eq!(i == 1, status[i as usize]);
    }

    for i in 0..SLOT_COUNT {
        assert_ok!((), pws.write_slot(i, "name", "login", "password"));
    }
    assert_ok!([true; SLOT_COUNT as usize], pws.get_slot_status());
}

#[test_device]
fn get_data(device: DeviceWrapper) {
    let mut device = device;
    let mut pws = get_pws(&mut device);
    assert_ok!((), pws.write_slot(1, "name", "login", "password"));
    assert_ok!("name".to_string(), pws.get_slot_name(1));
    assert_ok!("login".to_string(), pws.get_slot_login(1));
    assert_ok!("password".to_string(), pws.get_slot_password(1));

    assert_ok!((), pws.erase_slot(1));
    assert_cmd_err!(CommandError::SlotNotProgrammed, pws.get_slot_name(1));
    assert_cmd_err!(CommandError::SlotNotProgrammed, pws.get_slot_login(1));
    assert_cmd_err!(CommandError::SlotNotProgrammed, pws.get_slot_password(1));

    let name = "with å";
    let login = "pär@test.com";
    let password = "'i3lJc[09?I:,[u7dWz9";
    assert_ok!((), pws.write_slot(1, name, login, password));
    assert_ok!(name.to_string(), pws.get_slot_name(1));
    assert_ok!(login.to_string(), pws.get_slot_login(1));
    assert_ok!(password.to_string(), pws.get_slot_password(1));

    assert_lib_err!(LibraryError::InvalidSlot, pws.get_slot_name(SLOT_COUNT));
    assert_lib_err!(LibraryError::InvalidSlot, pws.get_slot_login(SLOT_COUNT));
    assert_lib_err!(LibraryError::InvalidSlot, pws.get_slot_password(SLOT_COUNT));
}

#[test_device]
fn write(device: DeviceWrapper) {
    let mut device = device;
    let mut pws = get_pws(&mut device);

    assert_lib_err!(
        LibraryError::InvalidSlot,
        pws.write_slot(SLOT_COUNT, "name", "login", "password")
    );

    assert_ok!((), pws.write_slot(0, "", "login", "password"));
    assert_cmd_err!(CommandError::SlotNotProgrammed, pws.get_slot_name(0));
    assert_ok!(String::from("login"), pws.get_slot_login(0));
    assert_ok!(String::from("password"), pws.get_slot_password(0));

    assert_ok!((), pws.write_slot(0, "name", "", "password"));
    assert_ok!(String::from("name"), pws.get_slot_name(0));
    assert_cmd_err!(CommandError::SlotNotProgrammed, pws.get_slot_login(0));
    assert_ok!(String::from("password"), pws.get_slot_password(0));

    assert_ok!((), pws.write_slot(0, "name", "login", ""));
    assert_ok!(String::from("name"), pws.get_slot_name(0));
    assert_ok!(String::from("login"), pws.get_slot_login(0));
    assert_cmd_err!(CommandError::SlotNotProgrammed, pws.get_slot_password(0));
}

#[test_device]
fn erase(device: DeviceWrapper) {
    let mut device = device;
    let mut pws = get_pws(&mut device);
    assert_lib_err!(LibraryError::InvalidSlot, pws.erase_slot(SLOT_COUNT));

    assert_ok!((), pws.write_slot(0, "name", "login", "password"));
    assert_ok!((), pws.erase_slot(0));
    assert_ok!((), pws.erase_slot(0));
    assert_cmd_err!(CommandError::SlotNotProgrammed, pws.get_slot_name(0));
}
