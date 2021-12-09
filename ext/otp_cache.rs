// otp_cache.rs

// Copyright (C) 2020-2024 The Nitrocli Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use std::fs;
use std::io::Write as _;
use std::path;

use anyhow::Context as _;
use clap::StructOpt as _;

mod ext;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct Cache {
  hotp: Vec<Slot>,
  totp: Vec<Slot>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct Slot {
  name: String,
  id: u8,
}

/// Access Nitrokey OTP slots by name
///
/// This command caches the names of the OTP slots on a Nitrokey device
/// and makes it possible to generate a one-time password from a slot
/// with a given name without knowing its index. It only queries the
/// names of the OTP slots if there is no cached data or if the
/// `--force-update` option is set. The cache includes the Nitrokey's
/// serial number so that it is possible to use it with multiple
/// devices.
#[derive(Debug, clap::StructOpt)]
#[structopt(bin_name = "nitrocli otp-cache")]
struct Args {
  /// Always query the slot data even if it is already cached
  #[structopt(short, long, global = true)]
  force_update: bool,
  #[structopt(subcommand)]
  cmd: Command,
}

#[derive(Debug, clap::StructOpt)]
enum Command {
  /// Generates a one-time password
  Get {
    /// The name of the OTP slot to generate a OTP from
    name: String,
  },
  /// Lists the cached slots and their names
  List,
}

fn main() -> anyhow::Result<()> {
  let args = Args::from_args();
  let ctx = ext::Context::from_env()?;

  let cache = get_cache(&ctx, args.force_update)?;
  match &args.cmd {
    Command::Get { name } => cmd_get(&ctx, &cache, name)?,
    Command::List => cmd_list(&cache),
  }
  Ok(())
}

fn cmd_get(ctx: &ext::Context, cache: &Cache, slot_name: &str) -> anyhow::Result<()> {
  let totp_slots = cache
    .totp
    .iter()
    .filter(|s| s.name == slot_name)
    .collect::<Vec<_>>();
  let hotp_slots = cache
    .hotp
    .iter()
    .filter(|s| s.name == slot_name)
    .collect::<Vec<_>>();
  if totp_slots.len() + hotp_slots.len() > 1 {
    Err(anyhow::anyhow!(
      "Found multiple OTP slots with the given name"
    ))
  } else if let Some(slot) = totp_slots.first() {
    generate_otp(ctx, "totp", slot.id)
  } else if let Some(slot) = hotp_slots.first() {
    generate_otp(ctx, "hotp", slot.id)
  } else {
    Err(anyhow::anyhow!("Found no OTP slot with the given name"))
  }
}

fn cmd_list(cache: &Cache) {
  println!("alg\tslot\tname");
  for slot in &cache.totp {
    println!("totp\t{}\t{}", slot.id, slot.name);
  }
  for slot in &cache.hotp {
    println!("hotp\t{}\t{}", slot.id, slot.name);
  }
}

fn get_cache(ctx: &ext::Context, force_update: bool) -> anyhow::Result<Cache> {
  let mut mgr = nitrokey::take().context("Failed to obtain Nitrokey manager instance")?;
  let device = ctx.connect(&mut mgr)?;
  let serial_number = get_serial_number(&device)?;
  let cache_file = ctx.cache_dir().join(format!("{}.toml", serial_number));

  if cache_file.is_file() && !force_update {
    load_cache(&cache_file)
  } else {
    let cache = get_otp_slots(&device)?;
    save_cache(&cache, &cache_file)?;
    Ok(cache)
  }
}

fn load_cache(path: &path::Path) -> anyhow::Result<Cache> {
  let s = fs::read_to_string(path).context("Failed to read cache file")?;
  toml::from_str(&s).context("Failed to parse cache file")
}

fn save_cache(cache: &Cache, path: &path::Path) -> anyhow::Result<()> {
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent).context("Failed to create cache parent directory")?;
  }
  let mut f = fs::File::create(path).context("Failed to create cache file")?;
  let data = toml::to_vec(cache).context("Failed to serialize cache")?;
  f.write_all(&data).context("Failed to write cache file")?;
  Ok(())
}

fn get_serial_number<'a>(device: &impl nitrokey::Device<'a>) -> anyhow::Result<String> {
  // TODO: Consider using hidapi serial number (if available)
  Ok(device.get_serial_number()?.to_string().to_lowercase())
}

fn get_otp_slots_fn<D, F>(device: &D, f: F) -> anyhow::Result<Vec<Slot>>
where
  D: nitrokey::GenerateOtp,
  F: Fn(&D, u8) -> Result<String, nitrokey::Error>,
{
  let mut slots = Vec::new();
  let mut slot = 0u8;
  loop {
    let result = f(device, slot);
    match result {
      Ok(name) => {
        slots.push(Slot { name, id: slot });
      }
      Err(nitrokey::Error::LibraryError(nitrokey::LibraryError::InvalidSlot)) => break,
      Err(nitrokey::Error::CommandError(nitrokey::CommandError::SlotNotProgrammed)) => {}
      Err(err) => return Err(err).context("Failed to check OTP slot"),
    }
    slot = slot
      .checked_add(1)
      .context("Encountered integer overflow when iterating OTP slots")?;
  }
  Ok(slots)
}

fn get_otp_slots(device: &impl nitrokey::GenerateOtp) -> anyhow::Result<Cache> {
  Ok(Cache {
    totp: get_otp_slots_fn(device, |device, slot| device.get_totp_slot_name(slot))?,
    hotp: get_otp_slots_fn(device, |device, slot| device.get_hotp_slot_name(slot))?,
  })
}

fn generate_otp(ctx: &ext::Context, algorithm: &str, slot: u8) -> anyhow::Result<()> {
  ctx
    .nitrocli()
    .args(["otp", "get"].iter())
    .arg(slot.to_string())
    .arg("--algorithm")
    .arg(algorithm)
    .spawn()
}
