extern crate cc;

use std::env;
use std::io;
use std::io::{Read, Write};
use std::fs;
use std::path;

struct Version {
    major: String,
    minor: String,
    git: String,
}

fn stringify(err: env::VarError) -> String {
    format!("{}", err)
}

fn extract_git_version(pre: &str) -> Result<String, String> {
    // If a pre-release version is set, it is expected to have the format
    // pre.v<maj>.<min>.<n>.g<hash>, where <maj> and <min> are the last major and minor version,
    // <n> is the number of commits since this version and <hash> is the hash of the last commit.
    let parts: Vec<&str> = pre.split('.').collect();
    if parts.len() != 5 {
        return Err(format!("'{}' is not a valid pre-release version", pre));
    }
    Ok(format!("{}.{}-{}-{}", parts[1], parts[2], parts[3], parts[4]))
}

fn get_version() -> Result<Version, String> {
    let major = env::var("CARGO_PKG_VERSION_MAJOR").map_err(stringify)?;
    let minor = env::var("CARGO_PKG_VERSION_MINOR").map_err(stringify)?;
    let patch = env::var("CARGO_PKG_VERSION_PATCH").map_err(stringify)?;
    let pre = env::var("CARGO_PKG_VERSION_PRE").map_err(stringify)?;

    let git = match pre.is_empty() {
        true => match patch.is_empty() {
            true => format!("v{}.{}", major, minor),
            false => format!("v{}.{}.{}", major, minor, patch),
        },
        false => extract_git_version(&pre)?,
    };

    Ok(Version {
        major,
        minor,
        git,
    })
}

fn prepare_version_source(
    version: &Version,
    out_path: &path::Path,
    library_path: &path::Path
) -> io::Result<path::PathBuf> {
    let out = out_path.join("version.cc");
    let template = library_path.join("version.cc.in");

    let mut file = fs::File::open(template)?;
    let mut data = String::new();
    file.read_to_string(&mut data)?;
    drop(file);

    let data = data
        .replace("@PROJECT_VERSION_MAJOR@", &version.major)
        .replace("@PROJECT_VERSION_MINOR@", &version.minor)
        .replace("@PROJECT_VERSION_GIT@", &version.git);

    let mut file = fs::File::create(&out)?;
    file.write_all(data.as_bytes())?;

    Ok(out)
}

fn main() {
    let out_dir = env::var("OUT_DIR").expect("Environment variable OUT_DIR is not set");
    let out_path = path::PathBuf::from(out_dir);

    let version = get_version().expect("Could not extract library version");

    let sources = [
        "DeviceCommunicationExceptions.cpp",
        "NK_C_API.cc",
        "NitrokeyManager.cc",
        "command_id.cc",
        "device.cc",
        "log.cc",
        "misc.cc",
    ];
    let library_dir = format!("libnitrokey-{}", version.git);
    let library_path = path::Path::new(&library_dir);

    let version_source = prepare_version_source(&version, &out_path, &library_path)
        .expect("Could not prepare the version source file");

    cc::Build::new()
        .cpp(true)
        .flag_if_supported("-std=c++14")
        .include(library_path.join("libnitrokey"))
        .files(sources.iter().map(|s| library_path.join(s)))
        .file(version_source)
        .compile("libnitrokey.a");

    let target = std::env::var("TARGET").unwrap();
    if let Some(_) = target.find("darwin") {
        println!("cargo:rustc-link-lib=hidapi");
    } else if let Some(_) = target.find("linux") {
        println!("cargo:rustc-link-lib=hidapi-libusb");
    }
}
