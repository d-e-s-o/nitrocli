use std::ffi::CStr;
use libc::c_int;
use sys::*;
use std::{error, fmt};

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Error {
	Initialized,
	NotFound,
	General,
	Write,
	Read,
	String(String),
}

pub type Result<T> = ::std::result::Result<T, Error>;

impl From<c_int> for Error {
	fn from(value: c_int) -> Error {
		match value {
			_ => Error::General
		}
	}
}

impl From<*mut hid_device> for Error {
	fn from(value: *mut hid_device) -> Error {
		unsafe {
			Error::String(CStr::from_ptr(hid_error(value) as *const _).to_str().unwrap().to_owned())
		}
	}
}

impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		f.write_str(error::Error::description(self))
	}
}

impl error::Error for Error {
	fn description(&self) -> &str {
		match *self {
			Error::Initialized =>
				"Already initialized.",

			Error::NotFound =>
				"Device not found.",

			Error::General =>
				"General error.",

			Error::Write =>
				"Write error.",

			Error::Read =>
				"Read error.",

			Error::String(ref err) =>
				err,
		}
	}
}
