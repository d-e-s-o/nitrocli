// main.rs

// Copyright (C) 2020-2021 The Nitrocli Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use std::fs;
use std::io::Write as _;
use std::path;

use anyhow::Context as _;

use structopt::StructOpt as _;

// TODO: query from user
const USER_PIN: &str = "123456";

#[derive(Debug, Default, serde::Deserialize, serde::Serialize)]
struct Cache {
  slots: Vec<Slot>,
}

impl Cache {
  pub fn find_slot(&self, name: &str) -> anyhow::Result<u8> {
    let slots = self
      .slots
      .iter()
      .filter(|s| s.name == name)
      .collect::<Vec<_>>();
    if slots.len() > 1 {
      Err(anyhow::anyhow!(
        "Found multiple PWS slots with the given name"
      ))
    } else if let Some(slot) = slots.first() {
      Ok(slot.id)
    } else {
      Err(anyhow::anyhow!("Found no PWS slot with the given name"))
    }
  }
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct Slot {
  name: String,
  id: u8,
}

/// Access Nitrokey PWS slots by name
///
/// This command caches the names of the PWS slots on a Nitrokey device
/// and makes it possible to fetch a login or a password from a slot
/// with a given name without knowing its index. It only queries the
/// names of the PWS slots if there is no cached data or if the
/// `--force-update` option is set. The cache includes the Nitrokey's
/// serial number so that it is possible to use it with multiple
/// devices.
#[derive(Debug, structopt::StructOpt)]
#[structopt(bin_name = "nitrocli pws-cache")]
struct Args {
  /// Always query the slot data even if it is already cached
  #[structopt(short, long)]
  force_update: bool,
  #[structopt(subcommand)]
  cmd: Command,
}

#[derive(Debug, structopt::StructOpt)]
enum Command {
  /// Fetches the login and the password from a PWS slot
  Get(GetArgs),
  /// Fetches the login from a PWS slot
  GetLogin(GetArgs),
  /// Fetches the password from a PWS slot
  GetPassword(GetArgs),
  /// Lists the cached slots and their names
  List,
}

#[derive(Debug, structopt::StructOpt)]
struct GetArgs {
  /// The name of the PWS slot to fetch
  name: String,
}

fn main() -> anyhow::Result<()> {
  let args = Args::from_args();
  let ctx = nitrocli_ext::Context::from_env()?;

  let cache = get_cache(&ctx, args.force_update)?;
  match &args.cmd {
    Command::Get(args) => cmd_get(&ctx, &cache, &args.name)?,
    Command::GetLogin(args) => cmd_get_login(&ctx, &cache, &args.name)?,
    Command::GetPassword(args) => cmd_get_password(&ctx, &cache, &args.name)?,
    Command::List => cmd_list(&cache),
  }
  Ok(())
}

fn cmd_get(ctx: &nitrocli_ext::Context, cache: &Cache, slot_name: &str) -> anyhow::Result<()> {
  let slot = cache.find_slot(slot_name)?;
  prepare_pws_get(ctx, slot)
    .arg("--login")
    .arg("--password")
    .spawn()
}

fn cmd_get_login(
  ctx: &nitrocli_ext::Context,
  cache: &Cache,
  slot_name: &str,
) -> anyhow::Result<()> {
  let slot = cache.find_slot(slot_name)?;
  prepare_pws_get(ctx, slot)
    .arg("--login")
    .arg("--quiet")
    .spawn()
}

fn cmd_get_password(
  ctx: &nitrocli_ext::Context,
  cache: &Cache,
  slot_name: &str,
) -> anyhow::Result<()> {
  let slot = cache.find_slot(slot_name)?;
  prepare_pws_get(ctx, slot)
    .arg("--password")
    .arg("--quiet")
    .spawn()
}

fn cmd_list(cache: &Cache) {
  println!("slot\tname");
  for slot in &cache.slots {
    println!("{}\t{}", slot.id, slot.name);
  }
}

fn get_cache(ctx: &nitrocli_ext::Context, force_update: bool) -> anyhow::Result<Cache> {
  let mut mgr = nitrokey::take().context("Failed to obtain Nitrokey manager instance")?;
  let mut device = ctx.connect(&mut mgr)?;
  let serial_number = get_serial_number(&device)?;
  let cache_file = ctx.cache_dir().join(&format!("{}.toml", serial_number));

  if cache_file.is_file() && !force_update {
    load_cache(&cache_file)
  } else {
    let cache = get_pws_slots(&mut device)?;
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

fn get_pws_slots<'a>(device: &mut impl nitrokey::GetPasswordSafe<'a>) -> anyhow::Result<Cache> {
  let pws = device
    .get_password_safe(USER_PIN)
    .context("Failed to open password safe")?;
  let slots = pws
    .get_slots()
    .context("Failed to query password safe slots")?;
  let mut cache = Cache::default();
  for slot in slots {
    if let Some(slot) = slot {
      let id = slot.index();
      let name = slot
        .get_name()
        .with_context(|| format!("Failed to query name for password slot {}", id))?;
      cache.slots.push(Slot { name, id });
    }
  }
  Ok(cache)
}

fn prepare_pws_get(ctx: &nitrocli_ext::Context, slot: u8) -> nitrocli_ext::Nitrocli {
  let mut ncli = ctx.nitrocli();
  let _ = ncli.args(&["pws", "get"]);
  let _ = ncli.arg(slot.to_string());
  ncli
}
