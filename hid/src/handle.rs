use std::time::Duration;

use libc::c_int;
use sys::*;
use error::{self, Error};

/// A device handle.
pub struct Handle {
	ptr: *mut hid_device,
}

impl Handle {
	#[doc(hidden)]
	pub unsafe fn new(ptr: *mut hid_device) -> Self {
		Handle {
			ptr: ptr,
		}
	}

	#[doc(hidden)]
	pub unsafe fn as_ptr(&self) -> *const hid_device {
		self.ptr as *const _
	}

	#[doc(hidden)]
	pub unsafe fn as_mut_ptr(&mut self) -> *mut hid_device {
		self.ptr
	}
}

impl Handle {
	/// Set the handle in blocking or non-blocking mode.
	pub fn blocking(&mut self, value: bool) -> error::Result<&mut Self> {
		unsafe {
			match hid_set_nonblocking(self.as_mut_ptr(), if value { 1 } else { 0 }) {
				0 =>
					Ok(self),

				_ =>
					Err(Error::General)
			}
		}
	}

	/// The data accessor.
	pub fn data(&mut self) -> Data {
		unsafe {
			Data::new(self)
		}
	}

	/// The feature accessor.
	pub fn feature(&mut self) -> Feature {
		unsafe {
			Feature::new(self)
		}
	}
}

/// The data accessor.
pub struct Data<'a> {
	handle: &'a mut Handle,
}

impl<'a> Data<'a> {
	#[doc(hidden)]
	pub unsafe fn new(handle: &mut Handle) -> Data {
		Data { handle: handle }
	}

	/// Write data to the device.
	///
	/// The first byte must be the report ID.
	pub fn write<T: AsRef<[u8]>>(&mut self, data: T) -> error::Result<usize> {
		let data = data.as_ref();

		unsafe {
			match hid_write(self.handle.as_mut_ptr(), data.as_ptr(), data.len()) {
				-1 =>
					Err(Error::Write),

				length =>
					Ok(length as usize)
			}
		}
	}

	/// Write data to the device with the given report ID.
	pub fn write_to<T: AsRef<[u8]>>(&mut self, id: u8, data: T) -> error::Result<usize> {
		let     data   = data.as_ref();
		let mut buffer = vec![0u8; data.len() + 1];

		buffer[0] = id;
		(&mut buffer[1 ..]).copy_from_slice(data);

		self.write(&buffer)
	}

	/// Read data from the device.
	///
	/// If the device supports reports the first byte will contain the report ID.
	///
	/// Returns the amount of read bytes or `None` if there was a timeout.
	pub fn read<T: AsMut<[u8]>>(&mut self, mut data: T, timeout: Duration) -> error::Result<Option<usize>> {
		let data   = data.as_mut();
		let result = if timeout.as_secs() == 0 && timeout.subsec_nanos() == 0 {
			unsafe {
				hid_read(self.handle.as_mut_ptr(), data.as_mut_ptr(), data.len())
			}
		}
		else {
			unsafe {
				// Timeout is in milliseconds.
				hid_read_timeout(self.handle.as_mut_ptr(), data.as_mut_ptr(), data.len(),
					timeout.as_secs() as c_int * 1_000 + timeout.subsec_nanos() as c_int / 1_000_000)
			}
		};

		match result {
			-1 =>
				Err(Error::Read),

			0 =>
				Ok(None),

			v =>
				Ok(Some(v as usize))
		}
	}

	/// Read data from the device.
	///
	/// Returns the report ID and the amount of read bytes or `None` if there was a timeout.
	pub fn read_from<T: AsMut<[u8]>>(&mut self, mut data: T, timeout: Duration) -> error::Result<Option<(u8, usize)>> {
		let     data   = data.as_mut();
		let mut buffer = Vec::with_capacity(data.len() + 1);

		if let Some(length) = self.read(&mut buffer, timeout)? {
			data[0..length - 1].copy_from_slice(&buffer[1..length]);

			Ok(Some((buffer[0], length - 1)))
		}
		else {
			Ok(None)
		}
	}
}

/// The feature accessor.
pub struct Feature<'a> {
	handle: &'a mut Handle,
}

impl<'a> Feature<'a> {
	#[doc(hidden)]
	pub unsafe fn new(handle: &mut Handle) -> Feature {
		Feature { handle: handle }
	}

	/// Send a feature request.
	///
	/// The first byte must be the report ID.
	pub fn send<T: AsRef<[u8]>>(&mut self, data: T) -> error::Result<usize> {
		let data = data.as_ref();

		unsafe {
			match hid_send_feature_report(self.handle.as_mut_ptr(), data.as_ptr(), data.len()) {
				-1 =>
					Err(Error::Write),

				length =>
					Ok(length as usize)
			}
		}
	}

	/// Send a feature request to the given report ID.
	pub fn send_to<T: AsRef<[u8]>>(&mut self, id: u8, data: T) -> error::Result<usize> {
		let     data   = data.as_ref();
		let mut buffer = Vec::with_capacity(data.len() + 1);

		buffer.push(id);
		buffer.extend(data);

		self.send(&buffer).map(|v| v - 1)
	}

	/// Get a feature request.
	///
	/// The first byte must be the report ID.
	pub fn get<T: AsMut<[u8]>>(&mut self, mut data: T) -> error::Result<usize> {
		let data = data.as_mut();

		unsafe {
			match hid_get_feature_report(self.handle.as_mut_ptr(), data.as_mut_ptr(), data.len()) {
				-1 =>
					Err(Error::Read),

				length =>
					Ok(length as usize)
			}
		}
	}

	/// Get a feature request from the given report ID.
	pub fn get_from<T: AsMut<[u8]>>(&mut self, id: u8, mut data: T) -> error::Result<usize> {
		let     data   = data.as_mut();
		let mut buffer = vec![0u8; data.len() + 1];
		buffer[0] = id;

		let length = self.get(&mut buffer)?;
		data[0..length - 1].copy_from_slice(&buffer[1..length]);

		Ok(length - 1)
	}
}

impl Drop for Handle {
	fn drop(&mut self) {
		unsafe {
			hid_close(self.as_mut_ptr());
		}
	}
}
