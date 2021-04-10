// main.rs

// Copyright (C) 2020-2021 The Nitrocli Developers
// SPDX-License-Identifier: GPL-3.0-or-later

mod ext;

use std::collections;
use std::fs;
use std::io;
use std::path;

use anyhow::Context as _;

type OtpCache = collections::BTreeMap<String, Vec<Slot>>;

#[derive(Debug, Default, serde::Deserialize, serde::Serialize)]
struct PwsCache {
  slots: Vec<Slot>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct Slot {
  name: String,
  id: usize,
}

/// Access Nitrokey OTP and PWS slots by name
#[derive(Debug, structopt::StructOpt)]
#[structopt(bin_name = "nitrocli cache")]
struct Args {
  #[structopt(subcommand)]
  cmd: Command,
}

#[derive(Debug, structopt::StructOpt)]
enum Command {
  /// Access the OTP cache
  Otp(OtpCommand),
  /// Access the PWS cache
  Pws(PwsCommand),
}

#[derive(Debug, structopt::StructOpt)]
enum OtpCommand {
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

#[derive(Debug, structopt::StructOpt)]
enum PwsCommand {
  /// Queries login from the PWS
  GetLogin {
    /// The name of the PWS slot to query
    name: String,
  },
  /// Queries password from the PWS
  GetPassword {
    /// The name of the PWS slot to query
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
  let ctx = ext::Context::from_env()?;

  let serial_number = get_serial_number(&ctx)?;
  let cache_dir = ctx.project_dirs.cache_dir().join(&serial_number);
  let cache_file = cache_dir.join(match &args.cmd {
    Command::Otp(_) => "otp.toml",
    Command::Pws(_) => "pws.toml",
  });

  match &args.cmd {
    Command::Otp(cmd) => match cmd {
      OtpCommand::Get { name } => {
        let cache: OtpCache = get_cache(&cache_file)?;
        for (algorithm, slots) in cache {
          if let Some(slot) = slots.iter().find(|s| &s.name == name) {
            generate_otp(&ctx, &algorithm, slot.id)?;
            return Ok(());
          }
        }
        anyhow::bail!("No OTP slot with the given name!");
      }
      OtpCommand::List => {
        let cache: OtpCache = get_cache(&cache_file)?;
        println!("alg\tslot\tname");
        for (algorithm, slots) in cache {
          for slot in slots {
            println!("{}\t{}\t{}", algorithm, slot.id, slot.name);
          }
        }
      }
      OtpCommand::Update => {
        let data = get_otp_slots(&ctx)?;
        save_cache(&data, &cache_file)?;
      }
    },
    Command::Pws(cmd) => match cmd {
      PwsCommand::GetLogin { name } => {
        let cache: PwsCache = get_cache(&cache_file)?;
        if let Some(slot) = cache.slots.iter().find(|s| &s.name == name) {
          get_pws_data(&ctx, slot.id, "--login")?;
        } else {
          anyhow::bail!("No PWS slot with the given name!");
        }
      }
      PwsCommand::GetPassword { name } => {
        let cache: PwsCache = get_cache(&cache_file)?;
        if let Some(slot) = cache.slots.iter().find(|s| &s.name == name) {
          get_pws_data(&ctx, slot.id, "--password")?;
        } else {
          anyhow::bail!("No PWS slot with the given name!");
        }
      }
      PwsCommand::List => {
        let cache: PwsCache = get_cache(&cache_file)?;
        println!("slot\tname");
        for slot in cache.slots {
          println!("{}\t{}", slot.id, slot.name);
        }
      }
      PwsCommand::Update => {
        let data = get_pws_slots(&ctx)?;
        save_cache(&data, &cache_file)?;
      }
    },
  }

  Ok(())
}

fn get_cache<T: serde::de::DeserializeOwned>(file: &path::Path) -> anyhow::Result<T> {
  if !file.is_file() {
    anyhow::bail!("There is no cached slot data.  Run the update command to initialize the cache.");
  }
  load_cache(&file)
}

fn load_cache<T: serde::de::DeserializeOwned>(path: &path::Path) -> anyhow::Result<T> {
  let s = fs::read_to_string(path).context("Failed to read cache file")?;
  toml::from_str(&s).context("Failed to parse cache file")
}

fn save_cache<T: serde::Serialize>(cache: &T, path: &path::Path) -> anyhow::Result<()> {
  use io::Write as _;

  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent).context("Failed to create cache parent directory")?;
  }
  let mut f = fs::File::create(path).context("Failed to create cache file")?;
  f.write_all(&toml::to_vec(cache).context("Failed to serialize cache")?)
    .context("Failed to write cache file")?;
  Ok(())
}

fn get_serial_number(ctx: &ext::Context) -> anyhow::Result<String> {
  let status = ext::Nitrocli::from_context(ctx)
    .arg("status")
    .text()
    .context("Failed to query device status")?;
  let r = regex::Regex::new(r#"(?m)^\s*serial number:\s*(\S.*)$"#)
    .context("Failed to compile serial regex")?;
  let captures = r
    .captures(&status)
    .context("Could not find serial number in status output")?;
  Ok(captures[1].to_lowercase())
}

fn get_otp_slots(ctx: &ext::Context) -> anyhow::Result<OtpCache> {
  let slots = ext::Nitrocli::from_context(ctx)
    .args(&["otp", "status"])
    .text()?;
  let mut cache = OtpCache::new();
  for line in slots.lines().skip(1) {
    let parts: Vec<_> = line.splitn(3, "\t").collect();
    if parts.len() == 3 {
      let algorithm = parts[0].to_owned();
      let id: usize = parts[1].parse().context("Failed to parse slot ID")?;
      let name = parts[2].to_owned();
      cache.entry(algorithm).or_default().push(Slot { name, id });
    }
  }
  Ok(cache)
}

fn generate_otp(ctx: &ext::Context, algorithm: &str, slot: usize) -> anyhow::Result<()> {
  ext::Nitrocli::from_context(ctx)
    .args(&["otp", "get"])
    .arg(slot.to_string())
    .arg("--algorithm")
    .arg(algorithm)
    .spawn()
}

fn get_pws_slots(ctx: &ext::Context) -> anyhow::Result<PwsCache> {
  let slots = ext::Nitrocli::from_context(ctx)
    .args(&["pws", "status"])
    .text()?;
  let mut cache = PwsCache::default();
  for line in slots.lines().skip(1) {
    let parts: Vec<_> = line.splitn(2, "\t").collect();
    if parts.len() == 2 {
      let id: usize = parts[0].parse().context("Failed to parse slot ID")?;
      let name = parts[1].to_owned();
      cache.slots.push(Slot { name, id });
    }
  }
  Ok(cache)
}

fn get_pws_data(ctx: &ext::Context, slot: usize, type_arg: &str) -> anyhow::Result<()> {
  ext::Nitrocli::from_context(ctx)
    .args(&["pws", "get", type_arg, "--quiet"])
    .arg(slot.to_string())
    .spawn()
}
