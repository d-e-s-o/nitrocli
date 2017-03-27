use std::sync::atomic::{AtomicBool, ATOMIC_BOOL_INIT, Ordering};

use sys::*;
use error::{self, Error};
use devices::Devices;

static INITIALIZED: AtomicBool = ATOMIC_BOOL_INIT;

/// The device manager.
pub struct Manager;

unsafe impl Send for Manager { }

/// Create the manager.
pub fn init() -> error::Result<Manager> {
	if INITIALIZED.load(Ordering::Relaxed) {
		return Err(Error::Initialized);
	}

	let status = unsafe { hid_init() };

	if status != 0 {
		return Err(Error::from(status));
	}

	INITIALIZED.store(true, Ordering::Relaxed);

	Ok(Manager)
}

impl Drop for Manager {
	fn drop(&mut self) {
		let status = unsafe { hid_exit() };

		if status != 0 {
			panic!("hid_exit() failed");
		}

		INITIALIZED.store(false, Ordering::Relaxed);
	}
}

impl Manager {
	/// Find the wanted device, `vendor` or `product` are given it will
	/// returns only the matches devices.
	pub fn find(&self, vendor: Option<u16>, product: Option<u16>) -> Devices {
		unsafe {
			Devices::new(vendor, product)
		}
	}

	/// Return all devices.
	pub fn devices(&self) -> Devices {
		self.find(None, None)
	}
}
