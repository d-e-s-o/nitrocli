// main.rs

// Copyright (C) 2020 The Nitrocli Developers
// SPDX-License-Identifier: GPL-3.0-or-later

mod ext;

use std::collections;
use std::fs;
use std::io::Write as _;
use std::path;
use std::process;

use anyhow::Context as _;

use nitrokey::Device as _;
use nitrokey::GenerateOtp as _;

use structopt::StructOpt as _;

type Cache = collections::BTreeMap<ext::OtpAlgorithm, Vec<Slot>>;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct Slot {
  index: u8,
  name: String,
}

/// Access Nitrokey OTP slots by name
#[derive(Debug, structopt::StructOpt)]
#[structopt(bin_name = "nitrocli otp-cache")]
struct Args {
  /// Update the cached slot data
  #[structopt(short, long)]
  force_update: bool,
  /// The OTP algorithm to use
  #[structopt(short, long, global = true, default_value = "totp")]
  algorithm: ext::OtpAlgorithm,
  #[structopt(subcommand)]
  cmd: Command,
}

#[derive(Debug, structopt::StructOpt)]
enum Command {
  /// Generates a one-time passwords
  Get {
    /// The name of the OTP slot to generate a OTP from
    name: String,
  },
  /// Lists the cached slots and their ID
  List,
}

fn main() -> anyhow::Result<()> {
  let args = Args::from_args();
  let ctx = ext::Context::from_env()?;
  let mut cache = get_cache(&ctx, &args)?;
  let slots = cache.remove(&args.algorithm).unwrap_or_default();

  match &args.cmd {
    Command::Get { name } => match slots.iter().find(|s| &s.name == name) {
      Some(slot) => print!("{}", generate_otp(&ctx, &args, slot.index)?),
      None => anyhow::bail!("No OTP slot with the given name!"),
    },
    Command::List => {
      println!("slot\tname");
      for slot in slots {
        println!("{}\t{}", slot.index, slot.name);
      }
    }
  }

  Ok(())
}

/// Instantiate a cache, either reading it from file or populating it
/// from live data (while also persisting it to a file).
fn get_cache(ctx: &ext::Context, args: &Args) -> anyhow::Result<Cache> {
  // TODO: If we keep invoking nitrokey-rs directly, it would be great
  //       to honor the verbosity and everything else nitrocli does.
  //       In that case perhaps a nitrocli-ext crate should provide a
  //       wrapper.
  let mut manager =
    nitrokey::take().context("Failed to acquire access to Nitrokey device manager")?;
  let device = manager
    .connect_model(ctx.model)
    .context("Failed to connect to Nitrokey device")?;

  let serial_number = device
    .get_serial_number()
    .context("Could not query the serial number")?;

  let project_dir =
    directories::ProjectDirs::from("", "", "nitrocli-otp-cache").ok_or_else(|| {
      anyhow::anyhow!("Could not determine the nitrocli-otp-cache application directory")
    })?;
  let cache_file = project_dir.cache_dir().join(format!(
    "{}-{}.toml",
    ctx.model.to_string().to_lowercase(),
    serial_number
  ));
  if args.force_update || !cache_file.is_file() {
    let cache = create_cache(&device, args)?;
    save_cache(&cache, &cache_file)
      .with_context(|| anyhow::anyhow!("Failed to save cache to {}", cache_file.display()))?;
    Ok(cache)
  } else {
    load_cache(&cache_file)
      .with_context(|| anyhow::anyhow!("Failed to load cache from {}", cache_file.display()))
  }
}

/// Create a cache based on data retrieved from the provided Nitrokey
/// device.
fn create_cache(device: &nitrokey::DeviceWrapper<'_>, args: &Args) -> anyhow::Result<Cache> {
  let mut cache = Cache::new();
  let mut slot = 0u8;
  loop {
    let result = match args.algorithm {
      ext::OtpAlgorithm::Hotp => device.get_hotp_slot_name(slot),
      ext::OtpAlgorithm::Totp => device.get_totp_slot_name(slot),
    };
    slot = slot
      .checked_add(1)
      .context("Encountered integer overflow when iterating OTP slots")?;
    match result {
      Ok(name) => cache.entry(args.algorithm).or_default().push(Slot {
        index: slot - 1,
        name,
      }),
      Err(nitrokey::Error::LibraryError(nitrokey::LibraryError::InvalidSlot)) => return Ok(cache),
      Err(nitrokey::Error::CommandError(nitrokey::CommandError::SlotNotProgrammed)) => (),
      Err(err) => return Err(err).context("Failed to check OTP slot"),
    }
  }
}

/// Save a cache to a file.
fn save_cache(cache: &Cache, path: &path::Path) -> anyhow::Result<()> {
  // There is guaranteed to exist a parent because our path is always
  // prefixed by the otp-cache directory.
  fs::create_dir_all(path.parent().unwrap()).context("Failed to create cache directory")?;

  let mut f = fs::File::create(path).context("Failed to create cache file")?;
  let toml = toml::to_vec(cache).context("Failed to convert cache data to TOML")?;
  f.write_all(&toml).context("Failed to write cache data")?;
  Ok(())
}

/// Load a cache from a file.
fn load_cache(path: &path::Path) -> anyhow::Result<Cache> {
  let s = fs::read_to_string(path)?;
  toml::from_str(&s).map_err(From::from)
}

fn generate_otp(ctx: &ext::Context, args: &Args, slot: u8) -> anyhow::Result<String> {
  // Attempt to prevent a "hang" of the Nitrokey by killing any scdaemon
  // that could currently have the device opened itself
  // (https://github.com/Nitrokey/libnitrokey/issues/137).
  let _ = process::Command::new("gpg-connect-agent")
    .stdout(process::Stdio::null())
    .stderr(process::Stdio::null())
    .arg("SCD KILLSCD")
    .arg("/bye")
    .output();

  ext::Nitrocli::from_context(ctx)
    .args(&["otp", "get"])
    .arg(slot.to_string())
    .arg("--algorithm")
    .arg(args.algorithm.to_string())
    .text()
}
