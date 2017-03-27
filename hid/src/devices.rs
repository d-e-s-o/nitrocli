use std::marker::PhantomData;

use sys::*;
use Device;

/// An iterator over the available devices.
pub struct Devices<'a> {
	ptr: *mut hid_device_info,
	cur: *mut hid_device_info,

	_marker: PhantomData<&'a ()>,
}

impl<'a> Devices<'a> {
	#[doc(hidden)]
	pub unsafe fn new(vendor: Option<u16>, product: Option<u16>) -> Self {
		let list = hid_enumerate(vendor.unwrap_or(0), product.unwrap_or(0));

		Devices {
			ptr: list,
			cur: list,

			_marker: PhantomData,
		}
	}
}

impl<'a> Iterator for Devices<'a> {
	type Item = Device<'a>;

	fn next(&mut self) -> Option<Self::Item> {
		if self.cur.is_null() {
			return None;
		}

		unsafe {
			let info = Device::new(self.cur);
			self.cur = (*self.cur).next;

			Some(info)
		}
	}
}

impl<'a> Drop for Devices<'a> {
	fn drop(&mut self) {
		unsafe {
			hid_free_enumeration(self.ptr);
		}
	}
}
