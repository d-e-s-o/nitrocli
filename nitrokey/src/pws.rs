use device::{Device, DeviceWrapper, Pro, Storage};
use libc;
use nitrokey_sys;
use util::{get_command_result, get_cstring, get_last_error, result_from_string, CommandError};

/// The number of slots in a [`PasswordSafe`][].
///
/// [`PasswordSafe`]: struct.PasswordSafe.html
pub const SLOT_COUNT: u8 = 16;

/// A password safe on a Nitrokey device.
///
/// The password safe stores a tuple consisting of a name, a login and a password on a slot.  The
/// number of available slots is [`SLOT_COUNT`][].  The slots are addressed starting with zero.  To
/// retrieve a password safe from a Nitrokey device, use the [`get_password_safe`][] method from
/// the [`GetPasswordSafe`][] trait.  Note that the device must live at least as long as the
/// password safe.
///
/// Once the password safe has been unlocked, it can be accessed without a password.  Therefore it
/// is mandatory to call [`lock`][] on the corresponding device after the password store is used.
/// As this command may have side effects on the Nitrokey Storage, it cannot be called
/// automatically once the password safe is destroyed.
///
/// # Examples
///
/// Open a password safe and access a password:
///
/// ```no_run
/// use nitrokey::{Device, GetPasswordSafe, PasswordSafe};
/// # use nitrokey::CommandError;
///
/// fn use_password_safe(pws: &PasswordSafe) -> Result<(), CommandError> {
///     let name = pws.get_slot_name(0)?;
///     let login = pws.get_slot_login(0)?;
///     let password = pws.get_slot_login(0)?;
///     println!("Credentials for {}: login {}, password {}", name, login, password);
///     Ok(())
/// }
///
/// # fn try_main() -> Result<(), CommandError> {
/// let device = nitrokey::connect()?;
/// let pws = device.get_password_safe("123456")?;
/// use_password_safe(&pws);
/// device.lock()?;
/// #     Ok(())
/// # }
/// ```
///
/// [`SLOT_COUNT`]: constant.SLOT_COUNT.html
/// [`get_password_safe`]: trait.GetPasswordSafe.html#method.get_password_safe
/// [`lock`]: trait.Device.html#method.lock
/// [`GetPasswordSafe`]: trait.GetPasswordSafe.html
pub struct PasswordSafe<'a> {
    _device: &'a Device,
}

/// Provides access to a [`PasswordSafe`][].
///
/// The device that implements this trait must always live at least as long as a password safe
/// retrieved from it.
///
/// [`PasswordSafe`]: struct.PasswordSafe.html
pub trait GetPasswordSafe {
    /// Enables and returns the password safe.
    ///
    /// The underlying device must always live at least as long as a password safe retrieved from
    /// it.  It is mandatory to lock the underlying device using [`lock`][] after the password safe
    /// has been used.  Otherwise, other applications can access the password store without
    /// authentication.
    ///
    /// # Errors
    ///
    /// - [`InvalidString`][] if one of the provided passwords contains a null byte
    /// - [`WrongPassword`][] if the current user password is wrong
    ///
    /// # Example
    ///
    /// ```no_run
    /// use nitrokey::{Device, GetPasswordSafe, PasswordSafe};
    /// # use nitrokey::CommandError;
    ///
    /// fn use_password_safe(pws: &PasswordSafe) {}
    ///
    /// # fn try_main() -> Result<(), CommandError> {
    /// let device = nitrokey::connect()?;
    /// match device.get_password_safe("123456") {
    ///     Ok(pws) => {
    ///         use_password_safe(&pws);
    ///         device.lock()?;
    ///     },
    ///     Err(err) => println!("Could not open the password safe: {}", err),
    /// };
    /// #     Ok(())
    /// # }
    /// ```
    ///
    /// [`device`]: struct.PasswordSafe.html#method.device
    /// [`lock`]: trait.Device.html#method.lock
    /// [`InvalidString`]: enum.CommandError.html#variant.InvalidString
    /// [`WrongPassword`]: enum.CommandError.html#variant.WrongPassword
    fn get_password_safe(&self, user_pin: &str) -> Result<PasswordSafe, CommandError>;
}

fn get_password_safe<'a>(
    device: &'a Device,
    user_pin: &str,
) -> Result<PasswordSafe<'a>, CommandError> {
    let user_pin_string = get_cstring(user_pin)?;
    let result = unsafe {
        get_command_result(nitrokey_sys::NK_enable_password_safe(
            user_pin_string.as_ptr(),
        ))
    };
    result.map(|()| PasswordSafe { _device: device })
}

impl<'a> PasswordSafe<'a> {
    /// Returns the status of all password slots.
    ///
    /// The status indicates whether a slot is programmed or not.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use nitrokey::{GetPasswordSafe, SLOT_COUNT};
    /// # use nitrokey::CommandError;
    ///
    /// # fn try_main() -> Result<(), CommandError> {
    /// let device = nitrokey::connect()?;
    /// let pws = device.get_password_safe("123456")?;
    /// pws.get_slot_status()?.iter().enumerate().for_each(|(slot, programmed)| {
    ///     let status = match *programmed {
    ///         true => "programmed",
    ///         false => "not programmed",
    ///     };
    ///     println!("Slot {}: {}", slot, status);
    /// });
    /// #     Ok(())
    /// # }
    /// ```
    pub fn get_slot_status(&self) -> Result<[bool; SLOT_COUNT as usize], CommandError> {
        let status_ptr = unsafe { nitrokey_sys::NK_get_password_safe_slot_status() };
        if status_ptr.is_null() {
            return Err(get_last_error());
        }
        let status_array_ptr = status_ptr as *const [u8; SLOT_COUNT as usize];
        let status_array = unsafe { *status_array_ptr };
        let mut result = [false; SLOT_COUNT as usize];
        for i in 0..SLOT_COUNT {
            result[i as usize] = status_array[i as usize] == 1;
        }
        unsafe {
            libc::free(status_ptr as *mut libc::c_void);
        }
        Ok(result)
    }

    /// Returns the name of the given slot (if it is programmed).
    ///
    /// # Errors
    ///
    /// - [`InvalidSlot`][] if the given slot is out of range
    /// - [`Unknown`][] if the slot is not programmed
    ///
    /// # Example
    ///
    /// ```no_run
    /// use nitrokey::GetPasswordSafe;
    /// # use nitrokey::CommandError;
    ///
    /// # fn try_main() -> Result<(), CommandError> {
    /// let device = nitrokey::connect()?;
    /// match device.get_password_safe("123456") {
    ///     Ok(pws) => {
    ///         let name = pws.get_slot_name(0)?;
    ///         let login = pws.get_slot_login(0)?;
    ///         let password = pws.get_slot_login(0)?;
    ///         println!("Credentials for {}: login {}, password {}", name, login, password);
    ///     },
    ///     Err(err) => println!("Could not open the password safe: {}", err),
    /// };
    /// #     Ok(())
    /// # }
    /// ```
    ///
    /// [`InvalidSlot`]: enum.CommandError.html#variant.InvalidSlot
    /// [`Unknown`]: enum.CommandError.html#variant.Unknown
    pub fn get_slot_name(&self, slot: u8) -> Result<String, CommandError> {
        unsafe { result_from_string(nitrokey_sys::NK_get_password_safe_slot_name(slot)) }
    }

    /// Returns the login for the given slot (if it is programmed).
    ///
    /// # Errors
    ///
    /// - [`InvalidSlot`][] if the given slot is out of range
    /// - [`Unknown`][] if the slot is not programmed
    ///
    /// # Example
    ///
    /// ```no_run
    /// use nitrokey::GetPasswordSafe;
    /// # use nitrokey::CommandError;
    ///
    /// # fn try_main() -> Result<(), CommandError> {
    /// let device = nitrokey::connect()?;
    /// let pws = device.get_password_safe("123456")?;
    /// let name = pws.get_slot_name(0)?;
    /// let login = pws.get_slot_login(0)?;
    /// let password = pws.get_slot_login(0)?;
    /// println!("Credentials for {}: login {}, password {}", name, login, password);
    /// #     Ok(())
    /// # }
    /// ```
    ///
    /// [`InvalidSlot`]: enum.CommandError.html#variant.InvalidSlot
    /// [`Unknown`]: enum.CommandError.html#variant.Unknown
    pub fn get_slot_login(&self, slot: u8) -> Result<String, CommandError> {
        unsafe { result_from_string(nitrokey_sys::NK_get_password_safe_slot_login(slot)) }
    }

    /// Returns the password for the given slot (if it is programmed).
    ///
    /// # Errors
    ///
    /// - [`InvalidSlot`][] if the given slot is out of range
    /// - [`Unknown`][] if the slot is not programmed
    ///
    /// # Example
    ///
    /// ```no_run
    /// use nitrokey::GetPasswordSafe;
    /// # use nitrokey::CommandError;
    ///
    /// # fn try_main() -> Result<(), CommandError> {
    /// let device = nitrokey::connect()?;
    /// let pws = device.get_password_safe("123456")?;
    /// let name = pws.get_slot_name(0)?;
    /// let login = pws.get_slot_login(0)?;
    /// let password = pws.get_slot_login(0)?;
    /// println!("Credentials for {}: login {}, password {}", name, login, password);
    /// #     Ok(())
    /// # }
    /// ```
    ///
    /// [`InvalidSlot`]: enum.CommandError.html#variant.InvalidSlot
    /// [`Unknown`]: enum.CommandError.html#variant.Unknown
    pub fn get_slot_password(&self, slot: u8) -> Result<String, CommandError> {
        unsafe { result_from_string(nitrokey_sys::NK_get_password_safe_slot_password(slot)) }
    }

    /// Writes the given slot with the given name, login and password.
    ///
    /// # Errors
    ///
    /// - [`InvalidSlot`][] if the given slot is out of range
    /// - [`InvalidString`][] if the provided token ID contains a null byte
    ///
    /// # Example
    ///
    /// ```no_run
    /// use nitrokey::GetPasswordSafe;
    /// # use nitrokey::CommandError;
    ///
    /// # fn try_main() -> Result<(), CommandError> {
    /// let device = nitrokey::connect()?;
    /// let pws = device.get_password_safe("123456")?;
    /// let name = pws.get_slot_name(0)?;
    /// let login = pws.get_slot_login(0)?;
    /// let password = pws.get_slot_login(0)?;
    /// println!("Credentials for {}: login {}, password {}", name, login, password);
    /// #     Ok(())
    /// # }
    /// ```
    ///
    /// [`InvalidSlot`]: enum.CommandError.html#variant.InvalidSlot
    /// [`InvalidString`]: enum.CommandError.html#variant.InvalidString
    pub fn write_slot(
        &self,
        slot: u8,
        name: &str,
        login: &str,
        password: &str,
    ) -> Result<(), CommandError> {
        let name_string = get_cstring(name)?;
        let login_string = get_cstring(login)?;
        let password_string = get_cstring(password)?;
        unsafe {
            get_command_result(nitrokey_sys::NK_write_password_safe_slot(
                slot,
                name_string.as_ptr(),
                login_string.as_ptr(),
                password_string.as_ptr(),
            ))
        }
    }

    /// Erases the given slot.  Erasing clears the stored name, login and password (if the slot was
    /// programmed).
    ///
    /// # Errors
    ///
    /// - [`InvalidSlot`][] if the given slot is out of range
    ///
    /// # Example
    ///
    /// ```no_run
    /// use nitrokey::GetPasswordSafe;
    /// # use nitrokey::CommandError;
    ///
    /// # fn try_main() -> Result<(), CommandError> {
    /// let device = nitrokey::connect()?;
    /// let pws = device.get_password_safe("123456")?;
    /// match pws.erase_slot(0) {
    ///     Ok(()) => println!("Erased slot 0."),
    ///     Err(err) => println!("Could not erase slot 0: {}", err),
    /// };
    /// #     Ok(())
    /// # }
    /// ```
    ///
    /// [`InvalidSlot`]: enum.CommandError.html#variant.InvalidSlot
    pub fn erase_slot(&self, slot: u8) -> Result<(), CommandError> {
        unsafe { get_command_result(nitrokey_sys::NK_erase_password_safe_slot(slot)) }
    }
}

impl<'a> Drop for PasswordSafe<'a> {
    fn drop(&mut self) {
        // TODO: disable the password safe -- NK_lock_device has side effects on the Nitrokey
        // Storage, see https://github.com/Nitrokey/nitrokey-storage-firmware/issues/65
    }
}

impl GetPasswordSafe for Pro {
    fn get_password_safe(&self, user_pin: &str) -> Result<PasswordSafe, CommandError> {
        get_password_safe(self, user_pin)
    }
}

impl GetPasswordSafe for Storage {
    fn get_password_safe(&self, user_pin: &str) -> Result<PasswordSafe, CommandError> {
        get_password_safe(self, user_pin)
    }
}

impl GetPasswordSafe for DeviceWrapper {
    fn get_password_safe(&self, user_pin: &str) -> Result<PasswordSafe, CommandError> {
        get_password_safe(self, user_pin)
    }
}
