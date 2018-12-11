use device::Device;
use nitrokey_sys;
use pws::{GetPasswordSafe, PasswordSafe, SLOT_COUNT};
use tests::util::{Target, ADMIN_PASSWORD, USER_PASSWORD};
use util::{result_from_string, CommandError};

fn get_pws(device: &Target) -> PasswordSafe {
    device.get_password_safe(USER_PASSWORD).unwrap()
}

#[test]
#[cfg_attr(not(any(feature = "test-pro", feature = "test-storage")), ignore)]
fn enable() {
    let device = Target::connect().unwrap();
    assert!(
        device
            .get_password_safe(&(USER_PASSWORD.to_owned() + "123"))
            .is_err()
    );
    assert!(device.get_password_safe(USER_PASSWORD).is_ok());
    assert!(device.get_password_safe(ADMIN_PASSWORD).is_err());
    assert!(device.get_password_safe(USER_PASSWORD).is_ok());
}

#[test]
#[cfg_attr(not(any(feature = "test-pro", feature = "test-storage")), ignore)]
fn drop() {
    let device = Target::connect().unwrap();
    {
        let pws = get_pws(&device);
        assert!(pws.write_slot(1, "name", "login", "password").is_ok());
        assert_eq!("name", pws.get_slot_name(1).unwrap());
        let result = result_from_string(unsafe { nitrokey_sys::NK_get_password_safe_slot_name(1) });
        assert_eq!(Ok(String::from("name")), result);
    }
    let result = result_from_string(unsafe { nitrokey_sys::NK_get_password_safe_slot_name(1) });
    assert_eq!(Ok(String::from("name")), result);
    assert!(device.lock().is_ok());
    let result = result_from_string(unsafe { nitrokey_sys::NK_get_password_safe_slot_name(1) });
    assert_eq!(Err(CommandError::NotAuthorized), result);
}

#[test]
#[cfg_attr(not(any(feature = "test-pro", feature = "test-storage")), ignore)]
fn get_status() {
    let device = Target::connect().unwrap();
    let pws = get_pws(&device);
    for i in 0..SLOT_COUNT {
        assert!(pws.erase_slot(i).is_ok(), "Could not erase slot {}", i);
    }
    let status = pws.get_slot_status().unwrap();
    assert_eq!(status, [false; SLOT_COUNT as usize]);

    assert!(pws.write_slot(1, "name", "login", "password").is_ok());
    let status = pws.get_slot_status().unwrap();
    for i in 0..SLOT_COUNT {
        assert_eq!(i == 1, status[i as usize]);
    }

    for i in 0..SLOT_COUNT {
        assert!(pws.write_slot(i, "name", "login", "password").is_ok());
    }
    let status = pws.get_slot_status().unwrap();
    assert_eq!(status, [true; SLOT_COUNT as usize]);
}

#[test]
#[cfg_attr(not(any(feature = "test-pro", feature = "test-storage")), ignore)]
fn get_data() {
    let device = Target::connect().unwrap();
    let pws = get_pws(&device);
    assert!(pws.write_slot(1, "name", "login", "password").is_ok());
    assert_eq!("name", pws.get_slot_name(1).unwrap());
    assert_eq!("login", pws.get_slot_login(1).unwrap());
    assert_eq!("password", pws.get_slot_password(1).unwrap());

    assert!(pws.erase_slot(1).is_ok());
    // TODO: check error codes
    assert_eq!(Err(CommandError::Unknown), pws.get_slot_name(1));
    assert_eq!(Err(CommandError::Unknown), pws.get_slot_login(1));
    assert_eq!(Err(CommandError::Unknown), pws.get_slot_password(1));

    let name = "with å";
    let login = "pär@test.com";
    let password = "'i3lJc[09?I:,[u7dWz9";
    assert!(pws.write_slot(1, name, login, password).is_ok());
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

#[test]
#[cfg_attr(not(any(feature = "test-pro", feature = "test-storage")), ignore)]
fn write() {
    let device = Target::connect().unwrap();
    let pws = get_pws(&device);

    assert_eq!(
        Err(CommandError::InvalidSlot),
        pws.write_slot(SLOT_COUNT, "name", "login", "password")
    );

    assert!(pws.write_slot(0, "", "login", "password").is_ok());
    assert_eq!(Err(CommandError::Unknown), pws.get_slot_name(0));
    assert_eq!(Ok(String::from("login")), pws.get_slot_login(0));
    assert_eq!(Ok(String::from("password")), pws.get_slot_password(0));

    assert!(pws.write_slot(0, "name", "", "password").is_ok());
    assert_eq!(Ok(String::from("name")), pws.get_slot_name(0));
    assert_eq!(Err(CommandError::Unknown), pws.get_slot_login(0));
    assert_eq!(Ok(String::from("password")), pws.get_slot_password(0));

    assert!(pws.write_slot(0, "name", "login", "").is_ok());
    assert_eq!(Ok(String::from("name")), pws.get_slot_name(0));
    assert_eq!(Ok(String::from("login")), pws.get_slot_login(0));
    assert_eq!(Err(CommandError::Unknown), pws.get_slot_password(0));
}

#[test]
#[cfg_attr(not(any(feature = "test-pro", feature = "test-storage")), ignore)]
fn erase() {
    let device = Target::connect().unwrap();
    let pws = get_pws(&device);
    assert_eq!(Err(CommandError::InvalidSlot), pws.erase_slot(SLOT_COUNT));

    assert!(pws.write_slot(0, "name", "login", "password").is_ok());
    assert!(pws.erase_slot(0).is_ok());
    assert!(pws.erase_slot(0).is_ok());
    assert_eq!(Err(CommandError::Unknown), pws.get_slot_name(0));
}
