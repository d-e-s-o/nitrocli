extern crate hid;

fn main() {
	let hid = hid::init().unwrap();

	for device in hid.devices() {
		print!("{} ", device.path().to_str().unwrap());
		print!("ID {:x}:{:x} ", device.vendor_id(), device.product_id());

		if let Some(name) = device.manufacturer_string() {
			print!("{} ", name);
		}

		if let Some(name) = device.product_string() {
			print!("{} ", name);
		}

		if let Some(name) = device.serial_number() {
			print!("{} ", name);
		}

		println!();
	}
}
