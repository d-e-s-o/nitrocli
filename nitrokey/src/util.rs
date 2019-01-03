use std::ffi::{CStr, CString};
use std::fmt;
use std::os::raw::{c_char, c_int};

use libc::{c_void, free};
use rand::Rng;

/// Error types returned by Nitrokey device or by the library.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CommandError {
    /// A packet with a wrong checksum has been sent or received.
    WrongCrc,
    /// A command tried to access an OTP slot that does not exist.
    WrongSlot,
    /// A command tried to generate an OTP on a slot that is not configured.
    SlotNotProgrammed,
    /// The provided password is wrong.
    WrongPassword,
    /// You are not authorized for this command or provided a wrong temporary
    /// password.
    NotAuthorized,
    /// An error occured when getting or setting the time.
    Timestamp,
    /// You did not provide a name for the OTP slot.
    NoName,
    /// This command is not supported by this device.
    NotSupported,
    /// This command is unknown.
    UnknownCommand,
    /// AES decryption failed.
    AesDecryptionFailed,
    /// An unknown error occured.
    Unknown,
    /// You passed a string containing a null byte.
    InvalidString,
    /// You passed an invalid slot.
    InvalidSlot,
    /// An error occured during random number generation.
    RngError,
}

/// Log level for libnitrokey.
///
/// Setting the log level to a lower level enables all output from higher levels too.  Currently,
/// only the log levels `Warning`, `DebugL1`, `Debug` and `DebugL2` are actually used.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LogLevel {
    /// Error messages.  Currently not used.
    Error,
    /// Warning messages.
    Warning,
    /// Informational messages.  Currently not used.
    Info,
    /// Basic debug messages, especially basic information on the sent and received packets.
    DebugL1,
    /// Detailed debug messages, especially detailed information on the sent and received packets.
    Debug,
    /// Very detailed debug messages, especially detailed information about the control flow for
    /// device communication (for example function entries and exits).
    DebugL2,
}

pub fn owned_str_from_ptr(ptr: *const c_char) -> String {
    unsafe {
        return CStr::from_ptr(ptr).to_string_lossy().into_owned();
    }
}

pub fn result_from_string(ptr: *const c_char) -> Result<String, CommandError> {
    if ptr.is_null() {
        return Err(CommandError::Unknown);
    }
    unsafe {
        let s = owned_str_from_ptr(ptr);
        free(ptr as *mut c_void);
        if s.is_empty() {
            return Err(get_last_error());
        }
        return Ok(s);
    }
}

pub fn get_command_result(value: c_int) -> Result<(), CommandError> {
    match value {
        0 => Ok(()),
        other => Err(CommandError::from(other)),
    }
}

pub fn get_last_result() -> Result<(), CommandError> {
    let value = unsafe { nitrokey_sys::NK_get_last_command_status() } as c_int;
    get_command_result(value)
}

pub fn get_last_error() -> CommandError {
    return match get_last_result() {
        Ok(()) => CommandError::Unknown,
        Err(err) => err,
    };
}

pub fn generate_password(length: usize) -> std::io::Result<Vec<u8>> {
    let mut data = vec![0u8; length];
    rand::thread_rng().fill(&mut data[..]);
    return Ok(data);
}

pub fn get_cstring<T: Into<Vec<u8>>>(s: T) -> Result<CString, CommandError> {
    CString::new(s).or(Err(CommandError::InvalidString))
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match *self {
            CommandError::WrongCrc => "A packet with a wrong checksum has been sent or received",
            CommandError::WrongSlot => "The given OTP slot does not exist",
            CommandError::SlotNotProgrammed => "The given OTP slot is not programmed",
            CommandError::WrongPassword => "The given password is wrong",
            CommandError::NotAuthorized => {
                "You are not authorized for this command or provided a wrong temporary password"
            }
            CommandError::Timestamp => "An error occured when getting or setting the time",
            CommandError::NoName => "You did not provide a name for the OTP slot",
            CommandError::NotSupported => "This command is not supported by this device",
            CommandError::UnknownCommand => "This command is unknown",
            CommandError::AesDecryptionFailed => "AES decryption failed",
            CommandError::Unknown => "An unknown error occured",
            CommandError::InvalidString => "You passed a string containing a null byte",
            CommandError::InvalidSlot => "The given slot is invalid",
            CommandError::RngError => "An error occured during random number generation",
        };
        write!(f, "{}", msg)
    }
}

impl From<c_int> for CommandError {
    fn from(value: c_int) -> Self {
        match value {
            1 => CommandError::WrongCrc,
            2 => CommandError::WrongSlot,
            3 => CommandError::SlotNotProgrammed,
            4 => CommandError::WrongPassword,
            5 => CommandError::NotAuthorized,
            6 => CommandError::Timestamp,
            7 => CommandError::NoName,
            8 => CommandError::NotSupported,
            9 => CommandError::UnknownCommand,
            10 => CommandError::AesDecryptionFailed,
            201 => CommandError::InvalidSlot,
            _ => CommandError::Unknown,
        }
    }
}

impl Into<i32> for LogLevel {
    fn into(self) -> i32 {
        match self {
            LogLevel::Error => 0,
            LogLevel::Warning => 1,
            LogLevel::Info => 2,
            LogLevel::DebugL1 => 3,
            LogLevel::Debug => 4,
            LogLevel::DebugL2 => 5,
        }
    }
}
