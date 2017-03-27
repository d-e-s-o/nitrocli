extern crate gcc;
extern crate pkg_config;

use std::env;
use std::io;
use std::path::PathBuf;
use std::process::Command;

fn main() {
	if env::var("CARGO_FEATURE_BUILD").is_err() {
		return;
	}

	fetch().unwrap();
	build().unwrap();

	println!("cargo:rustc-link-search=native={}", output().to_string_lossy());
}

fn output() -> PathBuf {
	PathBuf::from(env::var("OUT_DIR").unwrap())
}

fn source() -> PathBuf {
	output().join("hidapi")
}

fn fetch() -> io::Result<()> {
	Command::new("git")
		.current_dir(&output())
		.arg("clone")
		.arg("https://github.com/signal11/hidapi.git")
		.arg("hidapi")
		.status()?;

	Ok(())
}

#[cfg(target_os = "linux")]
fn build() -> io::Result<()> {
	let mut config = gcc::Config::new();

	config.file(source().join("libusb/hid.c"));
	config.include(source().join("hidapi"));

	for path in pkg_config::find_library("libusb-1.0").unwrap().include_paths {
		config.include(path.to_str().unwrap());
	}

	config.compile("libhidapi-libusb.a");

	Ok(())
}

#[cfg(target_os = "macos")]
fn build() -> io::Result<()> {
	let mut config = gcc::Config::new();

	config.file(source().join("libusb/hid.c"));
	config.include(source().join("hidapi"));

	for path in pkg_config::find_library("libusb-1.0").unwrap().include_paths {
		config.include(path.to_str().unwrap());
	}

	config.compile("libhidapi.a");

	Ok(())
}

#[cfg(target_os = "windows")]
fn build() -> io::Result<()> {
	let mut config = gcc::Config::new();

	config.file(source().join("windows/hid.c"));
	config.include(source().join("hidapi"));

	config.compile("libhidapi.a");

	Ok(())
}
