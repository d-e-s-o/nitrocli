use std::env;
use std::fs;
use std::io;
use std::io::{Read, Write};
use std::path;
use std::string;

use cc;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Version {
    major: u32,
    minor: u32,
    patch: Option<u32>,
}

impl string::ToString for Version {
    fn to_string(&self) -> String {
        match self.patch {
            Some(patch) => format!("v{}.{}.{}", self.major, self.minor, patch),
            None => format!("v{}.{}", self.major, self.minor),
        }
    }
}

const LIBNITROKEY_VERSION: Version = Version {
    major: 3,
    minor: 4,
    patch: Some(1),
};

fn prepare_version_source(
    version: Version,
    out_path: &path::Path,
    library_path: &path::Path,
) -> io::Result<path::PathBuf> {
    let out = out_path.join("version.cc");
    let template = library_path.join("version.cc.in");

    let mut file = fs::File::open(template)?;
    let mut data = String::new();
    file.read_to_string(&mut data)?;
    drop(file);

    let data = data
        .replace("@PROJECT_VERSION_MAJOR@", &version.major.to_string())
        .replace("@PROJECT_VERSION_MINOR@", &version.minor.to_string())
        .replace("@PROJECT_VERSION_GIT@", &version.to_string());

    let mut file = fs::File::create(&out)?;
    file.write_all(data.as_bytes())?;

    Ok(out)
}

fn main() {
    if env::var("USE_SYSTEM_LIBNITROKEY").is_ok() {
        println!("cargo:rustc-link-lib=nitrokey");
        return;
    }

    let out_dir = env::var("OUT_DIR").expect("Environment variable OUT_DIR is not set");
    let out_path = path::PathBuf::from(out_dir);

    let sources = [
        "DeviceCommunicationExceptions.cpp",
        "NK_C_API.cc",
        "NitrokeyManager.cc",
        "command_id.cc",
        "device.cc",
        "log.cc",
        "misc.cc",
    ];
    let library_dir = format!("libnitrokey-{}", LIBNITROKEY_VERSION.to_string());
    let library_path = path::Path::new(&library_dir);

    let version_source = prepare_version_source(LIBNITROKEY_VERSION, &out_path, &library_path)
        .expect("Could not prepare the version source file");

    cc::Build::new()
        .cpp(true)
        .flag("-std=c++14")
        .include(library_path.join("libnitrokey"))
        .files(sources.iter().map(|s| library_path.join(s)))
        .file(version_source)
        .compile("libnitrokey.a");

    let hidapi_library_name = if cfg!(target_os = "linux") {
        "hidapi-libusb"
    } else {
        "hidapi"
    };
    println!("cargo:rustc-link-lib={}", hidapi_library_name);
}
