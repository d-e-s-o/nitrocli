use std::marker::PhantomData;
use std::path::Path;
use std::ffi::CStr;

use sys::*;
use libc::{size_t, wchar_t, wcstombs};
use handle::Handle;
use error::{self, Error};

/// The HID device.
pub struct Device<'a> {
	ptr: *const hid_device_info,

	_marker: PhantomData<&'a ()>,
}

impl<'a> Device<'a> {
	#[doc(hidden)]
	pub unsafe fn new<'b>(ptr: *const hid_device_info) -> Device<'b> {
		Device {
			ptr: ptr,

			_marker: PhantomData,
		}
	}

	/// The path representation.
	pub fn path(&self) -> &Path {
		unsafe {
			Path::new(CStr::from_ptr((*self.ptr).path).to_str().unwrap())
		}
	}

	/// The vendor ID.
	pub fn vendor_id(&self) -> u16 {
		unsafe {
			(*self.ptr).vendor_id
		}
	}

	/// The product ID.
	pub fn product_id(&self) -> u16 {
		unsafe {
			(*self.ptr).product_id
		}
	}

	/// The serial number.
	pub fn serial_number(&self) -> Option<String> {
		unsafe {
			(*self.ptr).serial_number.as_ref().and_then(|p| to_string(p))
		}
	}

	/// The manufacturer string.
	pub fn manufacturer_string(&self) -> Option<String> {
		unsafe {
			(*self.ptr).manufacturer_string.as_ref().and_then(|p| to_string(p))
		}
	}

	/// The product string.
	pub fn product_string(&self) -> Option<String> {
		unsafe {
			(*self.ptr).product_string.as_ref().and_then(|p| to_string(p))
		}
	}

	/// The release number.
	pub fn release_number(&self) -> u16 {
		unsafe {
			(*self.ptr).release_number
		}
	}

	/// The usage page.
	pub fn usage_page(&self) -> u16 {
		unsafe {
			(*self.ptr).usage_page
		}
	}

	/// The usage number.
	pub fn usage(&self) -> u16 {
		unsafe {
			(*self.ptr).usage
		}
	}

	/// The interface number.
	pub fn interface_number(&self) -> isize {
		unsafe {
			(*self.ptr).interface_number as isize
		}
	}

	/// Opens the device to use it.
	pub fn open(&self) -> error::Result<Handle> {
		unsafe {
			let handle = hid_open((*self.ptr).vendor_id, (*self.ptr).product_id, (*self.ptr).serial_number);

			if handle.is_null() {
				return Err(Error::NotFound);
			}

			Ok(Handle::new(handle))
		}
	}
}

#[inline]
unsafe fn to_string(value: *const wchar_t) -> Option<String> {
	// USB descriptors are limited to 255 bytes.
	let mut buffer = [0u8; 256];
	let     length = wcstombs(buffer.as_mut_ptr() as *mut _, value, buffer.len());

	if length == size_t::max_value() {
		return None;
	}

	Some(String::from_utf8_lossy(&buffer[0.. length as usize]).into_owned())
}
