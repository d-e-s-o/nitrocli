mod util;

use std::ffi::CStr;

use libc::{c_int, c_void, free};
use nitrokey::{CommandError, Device, GetPasswordSafe, PasswordSafe, SLOT_COUNT};
use nitrokey_sys;
use nitrokey_test::test as test_device;

use crate::util::{ADMIN_PASSWORD, USER_PASSWORD};

fn get_slot_name_direct(slot: u8) -> Result<String, CommandError> {
    let ptr = unsafe { nitrokey_sys::NK_get_password_safe_slot_name(slot) };
    if ptr.is_null() {
        return Err(CommandError::Undefined);
    }
    let s = unsafe { CStr::from_ptr(ptr).to_string_lossy().into_owned() };
    unsafe { free(ptr as *mut c_void) };
    match s.is_empty() {
        true => {
            let error = unsafe { nitrokey_sys::NK_get_last_command_status() } as c_int;
            match error {
                0 => Ok(s),
                other => Err(CommandError::from(other)),
            }
        }
        false => Ok(s),
    }
}

fn get_pws<T>(device: &T) -> PasswordSafe
where
    T: Device,
{
    device.get_password_safe(USER_PASSWORD).unwrap()
}

#[test_device]
fn enable(device: DeviceWrapper) {
    assert!(device
        .get_password_safe(&(USER_PASSWORD.to_owned() + "123"))
        .is_err());
    assert!(device.get_password_safe(USER_PASSWORD).is_ok());
    assert!(device.get_password_safe(ADMIN_PASSWORD).is_err());
    assert!(device.get_password_safe(USER_PASSWORD).is_ok());
}

#[test_device]
fn drop(device: DeviceWrapper) {
    {
        let pws = get_pws(&device);
        assert_eq!(Ok(()), pws.write_slot(1, "name", "login", "password"));
        assert_eq!("name", pws.get_slot_name(1).unwrap());
        let result = get_slot_name_direct(1);
        assert_eq!(Ok(String::from("name")), result);
    }
    let result = get_slot_name_direct(1);
    assert_eq!(Ok(String::from("name")), result);
    assert_eq!(Ok(()), device.lock());
    let result = get_slot_name_direct(1);
    assert_eq!(Err(CommandError::NotAuthorized), result);
}

#[test_device]
fn get_status(device: DeviceWrapper) {
    let pws = get_pws(&device);
    for i in 0..SLOT_COUNT {
        assert_eq!(Ok(()), pws.erase_slot(i), "Could not erase slot {}", i);
    }
    let status = pws.get_slot_status().unwrap();
    assert_eq!(status, [false; SLOT_COUNT as usize]);

    assert_eq!(Ok(()), pws.write_slot(1, "name", "login", "password"));
    let status = pws.get_slot_status().unwrap();
    for i in 0..SLOT_COUNT {
        assert_eq!(i == 1, status[i as usize]);
    }

    for i in 0..SLOT_COUNT {
        assert_eq!(Ok(()), pws.write_slot(i, "name", "login", "password"));
    }
    let status = pws.get_slot_status().unwrap();
    assert_eq!(status, [true; SLOT_COUNT as usize]);
}

#[test_device]
fn get_data(device: DeviceWrapper) {
    let pws = get_pws(&device);
    assert_eq!(Ok(()), pws.write_slot(1, "name", "login", "password"));
    assert_eq!("name", pws.get_slot_name(1).unwrap());
    assert_eq!("login", pws.get_slot_login(1).unwrap());
    assert_eq!("password", pws.get_slot_password(1).unwrap());

    assert_eq!(Ok(()), pws.erase_slot(1));
    assert_eq!(Err(CommandError::SlotNotProgrammed), pws.get_slot_name(1));
    assert_eq!(Err(CommandError::SlotNotProgrammed), pws.get_slot_login(1));
    assert_eq!(
        Err(CommandError::SlotNotProgrammed),
        pws.get_slot_password(1)
    );

    let name = "with å";
    let login = "pär@test.com";
    let password = "'i3lJc[09?I:,[u7dWz9";
    assert_eq!(Ok(()), pws.write_slot(1, name, login, password));
    assert_eq!(name, pws.get_slot_name(1).unwrap());
    assert_eq!(login, pws.get_slot_login(1).unwrap());
    assert_eq!(password, pws.get_slot_password(1).unwrap());

    assert_eq!(
        Err(CommandError::InvalidSlot),
        pws.get_slot_name(SLOT_COUNT)
    );
    assert_eq!(
        Err(CommandError::InvalidSlot),
        pws.get_slot_login(SLOT_COUNT)
    );
    assert_eq!(
        Err(CommandError::InvalidSlot),
        pws.get_slot_password(SLOT_COUNT)
    );
}

#[test_device]
fn write(device: DeviceWrapper) {
    let pws = get_pws(&device);

    assert_eq!(
        Err(CommandError::InvalidSlot),
        pws.write_slot(SLOT_COUNT, "name", "login", "password")
    );

    assert_eq!(Ok(()), pws.write_slot(0, "", "login", "password"));
    assert_eq!(Err(CommandError::SlotNotProgrammed), pws.get_slot_name(0));
    assert_eq!(Ok(String::from("login")), pws.get_slot_login(0));
    assert_eq!(Ok(String::from("password")), pws.get_slot_password(0));

    assert_eq!(Ok(()), pws.write_slot(0, "name", "", "password"));
    assert_eq!(Ok(String::from("name")), pws.get_slot_name(0));
    assert_eq!(Err(CommandError::SlotNotProgrammed), pws.get_slot_login(0));
    assert_eq!(Ok(String::from("password")), pws.get_slot_password(0));

    assert_eq!(Ok(()), pws.write_slot(0, "name", "login", ""));
    assert_eq!(Ok(String::from("name")), pws.get_slot_name(0));
    assert_eq!(Ok(String::from("login")), pws.get_slot_login(0));
    assert_eq!(
        Err(CommandError::SlotNotProgrammed),
        pws.get_slot_password(0)
    );
}

#[test_device]
fn erase(device: DeviceWrapper) {
    let pws = get_pws(&device);
    assert_eq!(Err(CommandError::InvalidSlot), pws.erase_slot(SLOT_COUNT));

    assert_eq!(Ok(()), pws.write_slot(0, "name", "login", "password"));
    assert_eq!(Ok(()), pws.erase_slot(0));
    assert_eq!(Ok(()), pws.erase_slot(0));
    assert_eq!(Err(CommandError::SlotNotProgrammed), pws.get_slot_name(0));
}
