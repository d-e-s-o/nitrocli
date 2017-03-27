#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]

extern crate libc;

use libc::{c_void, c_ushort, wchar_t, c_int, c_uchar, size_t, c_char};

pub type hid_device = c_void;

#[repr(C)]
pub struct hid_device_info {
	pub path: *mut c_char,

	pub vendor_id:  c_ushort,
	pub product_id: c_ushort,

	pub serial_number:  *mut wchar_t,
	pub release_number: c_ushort,

	pub manufacturer_string: *mut wchar_t,
	pub product_string:      *mut wchar_t,

	pub usage_page: c_ushort,
	pub usage:      c_ushort,

	pub interface_number: c_int,

	pub next: *mut hid_device_info,
}

#[cfg_attr(target_os = "linux", link(name = "udev"))]
extern "C" { }

#[cfg_attr(target_os = "windows", link(name = "setupapi"))]
extern "C" { }

#[cfg_attr(all(feature = "static", target_os = "linux"), link(name = "hidapi-libusb", kind = "static"))]
#[cfg_attr(all(not(feature = "static"), target_os = "linux"), link(name = "hidapi-libusb"))]
#[cfg_attr(all(feature = "static", not(target_os = "linux")), link(name = "hidapi", kind = "static"))]
#[cfg_attr(all(not(feature = "static"), not(target_os = "linux")), link(name = "hidapi"))]
extern "C" {
	pub fn hid_init() -> c_int;
	pub fn hid_exit() -> c_int;

	pub fn hid_enumerate(vendor_id: c_ushort, product_id: c_ushort) -> *mut hid_device_info;
	pub fn hid_free_enumeration(devs: *mut hid_device_info);

	pub fn hid_open(vendor_id: c_ushort, product_id: c_ushort, serial_number: *const wchar_t) -> *mut hid_device;
	pub fn hid_open_path(path: *const c_char) -> *mut hid_device;

	pub fn hid_write(device: *mut hid_device, data: *const c_uchar, length: size_t) -> c_int;

	pub fn hid_read_timeout(device: *mut hid_device, data: *mut c_uchar, length: size_t, milleseconds: c_int) -> c_int;
	pub fn hid_read(device: *mut hid_device, data: *mut c_uchar, length: size_t) -> c_int;

	pub fn hid_set_nonblocking(device: *mut hid_device, nonblock: c_int) -> c_int;

	pub fn hid_send_feature_report(device: *mut hid_device, data: *const c_uchar, length: size_t) -> c_int;
	pub fn hid_get_feature_report(device: *mut hid_device, data: *mut c_uchar, length: size_t) -> c_int;

	pub fn hid_close(device: *mut hid_device);

	pub fn hid_get_manufacturer_string(device: *mut hid_device, string: *mut wchar_t, maxlen: size_t) -> c_int;
	pub fn hid_get_product_string(device: *mut hid_device, string: *mut wchar_t, maxlen: size_t) -> c_int;
	pub fn hid_get_serial_number_string(device: *mut hid_device, string: *mut wchar_t, maxlen: size_t) -> c_int;
	pub fn hid_get_indexed_string(device: *mut hid_device, string_index: c_int, string: *mut wchar_t, maxlen: size_t) -> c_int;

	pub fn hid_error(device: *mut hid_device) -> *const wchar_t;
}
