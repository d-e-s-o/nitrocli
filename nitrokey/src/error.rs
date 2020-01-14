// Copyright (C) 2019 Robin Krahl <robin.krahl@ireas.org>
// SPDX-License-Identifier: MIT

use std::error;
use std::fmt;
use std::os::raw;
use std::str;
use std::sync;

use crate::device;

/// An error returned by the nitrokey crate.
#[derive(Debug)]
pub enum Error {
    /// An error reported by the Nitrokey device in the response packet.
    CommandError(CommandError),
    /// A device communication error.
    CommunicationError(CommunicationError),
    /// An error occurred due to concurrent access to the Nitrokey device.
    ConcurrentAccessError,
    /// A library usage error.
    LibraryError(LibraryError),
    /// An error that occurred due to a poisoned lock.
    PoisonError(sync::PoisonError<sync::MutexGuard<'static, crate::Manager>>),
    /// An error that occurred during random number generation.
    RandError(Box<dyn error::Error>),
    /// An error that is caused by an unexpected value returned by libnitrokey.
    UnexpectedError,
    /// An unknown error returned by libnitrokey.
    UnknownError(i64),
    /// An error caused by a Nitrokey model that is not supported by this crate.
    UnsupportedModelError,
    /// An error occurred when interpreting a UTF-8 string.
    Utf8Error(str::Utf8Error),
}

impl From<raw::c_int> for Error {
    fn from(code: raw::c_int) -> Self {
        if let Some(err) = CommandError::try_from(code) {
            Error::CommandError(err)
        } else if let Some(err) = CommunicationError::try_from(256 - code) {
            Error::CommunicationError(err)
        } else if let Some(err) = LibraryError::try_from(code) {
            Error::LibraryError(err)
        } else {
            Error::UnknownError(code.into())
        }
    }
}

impl From<CommandError> for Error {
    fn from(err: CommandError) -> Self {
        Error::CommandError(err)
    }
}

impl From<CommunicationError> for Error {
    fn from(err: CommunicationError) -> Self {
        Error::CommunicationError(err)
    }
}

impl From<LibraryError> for Error {
    fn from(err: LibraryError) -> Self {
        Error::LibraryError(err)
    }
}

impl From<str::Utf8Error> for Error {
    fn from(error: str::Utf8Error) -> Self {
        Error::Utf8Error(error)
    }
}

impl From<sync::PoisonError<sync::MutexGuard<'static, crate::Manager>>> for Error {
    fn from(error: sync::PoisonError<sync::MutexGuard<'static, crate::Manager>>) -> Self {
        Error::PoisonError(error)
    }
}

impl From<sync::TryLockError<sync::MutexGuard<'static, crate::Manager>>> for Error {
    fn from(error: sync::TryLockError<sync::MutexGuard<'static, crate::Manager>>) -> Self {
        match error {
            sync::TryLockError::Poisoned(err) => err.into(),
            sync::TryLockError::WouldBlock => Error::ConcurrentAccessError,
        }
    }
}

impl<'a, T: device::Device<'a>> From<(T, Error)> for Error {
    fn from((_, err): (T, Error)) -> Self {
        err
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            Error::CommandError(ref err) => Some(err),
            Error::CommunicationError(ref err) => Some(err),
            Error::ConcurrentAccessError => None,
            Error::LibraryError(ref err) => Some(err),
            Error::PoisonError(ref err) => Some(err),
            Error::RandError(ref err) => Some(err.as_ref()),
            Error::UnexpectedError => None,
            Error::UnknownError(_) => None,
            Error::UnsupportedModelError => None,
            Error::Utf8Error(ref err) => Some(err),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Error::CommandError(ref err) => write!(f, "Command error: {}", err),
            Error::CommunicationError(ref err) => write!(f, "Communication error: {}", err),
            Error::ConcurrentAccessError => write!(f, "Internal error: concurrent access"),
            Error::LibraryError(ref err) => write!(f, "Library error: {}", err),
            Error::PoisonError(_) => write!(f, "Internal error: poisoned lock"),
            Error::RandError(ref err) => write!(f, "RNG error: {}", err),
            Error::UnexpectedError => write!(f, "An unexpected error occurred"),
            Error::UnknownError(ref err) => write!(f, "Unknown error: {}", err),
            Error::UnsupportedModelError => write!(f, "Unsupported Nitrokey model"),
            Error::Utf8Error(ref err) => write!(f, "UTF-8 error: {}", err),
        }
    }
}

/// An error reported by the Nitrokey device in the response packet.
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
    /// An error occurred when getting or setting the time.
    Timestamp,
    /// You did not provide a name for the OTP slot.
    NoName,
    /// This command is not supported by this device.
    NotSupported,
    /// This command is unknown.
    UnknownCommand,
    /// AES decryption failed.
    AesDecryptionFailed,
}

impl CommandError {
    fn try_from(value: raw::c_int) -> Option<Self> {
        match value {
            1 => Some(CommandError::WrongCrc),
            2 => Some(CommandError::WrongSlot),
            3 => Some(CommandError::SlotNotProgrammed),
            4 => Some(CommandError::WrongPassword),
            5 => Some(CommandError::NotAuthorized),
            6 => Some(CommandError::Timestamp),
            7 => Some(CommandError::NoName),
            8 => Some(CommandError::NotSupported),
            9 => Some(CommandError::UnknownCommand),
            10 => Some(CommandError::AesDecryptionFailed),
            _ => None,
        }
    }
}

impl error::Error for CommandError {}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match *self {
            CommandError::WrongCrc => "A packet with a wrong checksum has been sent or received",
            CommandError::WrongSlot => "The given slot does not exist",
            CommandError::SlotNotProgrammed => "The given slot is not programmed",
            CommandError::WrongPassword => "The given password is wrong",
            CommandError::NotAuthorized => {
                "You are not authorized for this command or provided a wrong temporary \
                 password"
            }
            CommandError::Timestamp => "An error occurred when getting or setting the time",
            CommandError::NoName => "You did not provide a name for the slot",
            CommandError::NotSupported => "This command is not supported by this device",
            CommandError::UnknownCommand => "This command is unknown",
            CommandError::AesDecryptionFailed => "AES decryption failed",
        })
    }
}

/// A device communication error.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CommunicationError {
    /// Could not connect to a Nitrokey device.
    NotConnected,
    /// Sending a packet failed.
    SendingFailure,
    /// Receiving a packet failed.
    ReceivingFailure,
    /// A packet with a wrong checksum was received.
    InvalidCrc,
}

impl CommunicationError {
    fn try_from(value: raw::c_int) -> Option<Self> {
        match value {
            2 => Some(CommunicationError::NotConnected),
            3 => Some(CommunicationError::SendingFailure),
            4 => Some(CommunicationError::ReceivingFailure),
            5 => Some(CommunicationError::InvalidCrc),
            _ => None,
        }
    }
}

impl error::Error for CommunicationError {}

impl fmt::Display for CommunicationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match *self {
            CommunicationError::NotConnected => "Could not connect to a Nitrokey device",
            CommunicationError::SendingFailure => "Sending a packet failed",
            CommunicationError::ReceivingFailure => "Receiving a packet failed",
            CommunicationError::InvalidCrc => "A packet with a wrong checksum was received",
        })
    }
}

/// A library usage error.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LibraryError {
    /// A supplied string exceeded a length limit.
    StringTooLong,
    /// You passed an invalid slot.
    InvalidSlot,
    /// The supplied string was not in hexadecimal format.
    InvalidHexString,
    /// The target buffer was smaller than the source.
    TargetBufferTooSmall,
    /// You passed a string containing a null byte.
    InvalidString,
}

impl LibraryError {
    fn try_from(value: raw::c_int) -> Option<Self> {
        match value {
            200 => Some(LibraryError::StringTooLong),
            201 => Some(LibraryError::InvalidSlot),
            202 => Some(LibraryError::InvalidHexString),
            203 => Some(LibraryError::TargetBufferTooSmall),
            _ => None,
        }
    }
}

impl error::Error for LibraryError {}

impl fmt::Display for LibraryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match *self {
            LibraryError::StringTooLong => "The supplied string is too long",
            LibraryError::InvalidSlot => "The given slot is invalid",
            LibraryError::InvalidHexString => "The supplied string is not in hexadecimal format",
            LibraryError::TargetBufferTooSmall => "The target buffer is too small",
            LibraryError::InvalidString => "You passed a string containing a null byte",
        })
    }
}
