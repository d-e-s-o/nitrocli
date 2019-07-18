// Copyright (C) 2018-2019 Robin Krahl <robin.krahl@ireas.org>
// SPDX-License-Identifier: MIT

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};

use libc::{c_void, free};
use rand_core::RngCore;
use rand_os::OsRng;

use crate::error::{Error, LibraryError};

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

pub fn owned_str_from_ptr(ptr: *const c_char) -> Result<String, Error> {
    unsafe { CStr::from_ptr(ptr) }
        .to_str()
        .map(String::from)
        .map_err(Error::from)
}

pub fn result_from_string(ptr: *const c_char) -> Result<String, Error> {
    if ptr.is_null() {
        return Err(Error::UnexpectedError);
    }
    let s = owned_str_from_ptr(ptr)?;
    unsafe { free(ptr as *mut c_void) };
    // An empty string can both indicate an error or be a valid return value.  In this case, we
    // have to check the last command status to decide what to return.
    if s.is_empty() {
        get_last_result().map(|_| s)
    } else {
        Ok(s)
    }
}

pub fn get_command_result(value: c_int) -> Result<(), Error> {
    if value == 0 {
        Ok(())
    } else {
        Err(Error::from(value))
    }
}

pub fn get_last_result() -> Result<(), Error> {
    get_command_result(unsafe { nitrokey_sys::NK_get_last_command_status() }.into())
}

pub fn get_last_error() -> Error {
    match get_last_result() {
        Ok(()) => Error::UnexpectedError,
        Err(err) => err,
    }
}

pub fn generate_password(length: usize) -> Result<Vec<u8>, Error> {
    let mut rng = OsRng::new().map_err(|err| Error::RandError(Box::new(err)))?;
    let mut data = vec![0u8; length];
    rng.fill_bytes(&mut data[..]);
    Ok(data)
}

pub fn get_cstring<T: Into<Vec<u8>>>(s: T) -> Result<CString, Error> {
    CString::new(s).or_else(|_| Err(LibraryError::InvalidString.into()))
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
