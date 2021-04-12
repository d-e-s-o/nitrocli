// main.rs

// Copyright (C) 2020-2021 The Nitrocli Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use std::fs;
use std::io;
use std::path;

use anyhow::Context as _;

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
#[derive(Debug, structopt::StructOpt)]
#[structopt(bin_name = "nitrocli cache")]
struct Args {
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
  /// Lists the cached slots and their names
  List,
  /// Updates the cached slot data
  Update,
}

fn main() -> anyhow::Result<()> {
  use structopt::StructOpt as _;

  let args = Args::from_args();
  let ctx = nitrocli_ext::Context::from_env("nitrocli-otp-cache")?;

  let mut mgr = nitrokey::take()?;
  let device = ctx.connect(&mut mgr)?;

  let serial_number = get_serial_number(&device)?;
  let cache_file = ctx
    .project_dirs
    .cache_dir()
    .join(&format!("{}.toml", serial_number));

  match &args.cmd {
    Command::Get { name } => {
      drop(device);
      drop(mgr);
      cmd_get(&ctx, &cache_file, name)
    }
    Command::List => cmd_list(&cache_file),
    Command::Update => cmd_update(&cache_file, &device),
  }
}

fn cmd_get(
  ctx: &nitrocli_ext::Context,
  cache_file: &path::Path,
  slot_name: &str,
) -> anyhow::Result<()> {
  let cache = get_cache(cache_file)?;
  let totp_slots: Vec<_> = cache.totp.iter().filter(|s| s.name == slot_name).collect();
  let hotp_slots: Vec<_> = cache.hotp.iter().filter(|s| s.name == slot_name).collect();
  if totp_slots.len() + hotp_slots.len() > 1 {
    Err(anyhow::anyhow!("Multiple OTP slots with the given name"))
  } else if let Some(slot) = totp_slots.first() {
    generate_otp(&ctx, "totp", slot.id)
  } else if let Some(slot) = hotp_slots.first() {
    generate_otp(&ctx, "hotp", slot.id)
  } else {
    Err(anyhow::anyhow!("No OTP slot with the given name"))
  }
}

fn cmd_list(cache_file: &path::Path) -> anyhow::Result<()> {
  let cache = get_cache(&cache_file)?;
  println!("alg\tslot\tname");
  for slot in cache.totp {
    println!("totp\t{}\t{}", slot.id, slot.name);
  }
  for slot in cache.hotp {
    println!("hotp\t{}\t{}", slot.id, slot.name);
  }
  Ok(())
}

fn cmd_update(cache_file: &path::Path, device: &impl nitrokey::GenerateOtp) -> anyhow::Result<()> {
  save_cache(&get_otp_slots(device)?, &cache_file)
}

fn get_cache(file: &path::Path) -> anyhow::Result<Cache> {
  if !file.is_file() {
    anyhow::bail!("There is no cached slot data.  Run the update command to initialize the cache.");
  }
  load_cache(&file)
}

fn load_cache(path: &path::Path) -> anyhow::Result<Cache> {
  let s = fs::read_to_string(path).context("Failed to read cache file")?;
  toml::from_str(&s).context("Failed to parse cache file")
}

fn save_cache(cache: &Cache, path: &path::Path) -> anyhow::Result<()> {
  use io::Write as _;

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
  let mut slot: u8 = 0;
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

fn generate_otp(ctx: &nitrocli_ext::Context, algorithm: &str, slot: u8) -> anyhow::Result<()> {
  ctx
    .nitrocli()
    .args(&["otp", "get"])
    .arg(slot.to_string())
    .arg("--algorithm")
    .arg(algorithm)
    .spawn()
}
