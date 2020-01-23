// config.rs

// *************************************************************************
// * Copyright (C) 2020 Daniel Mueller (deso@posteo.net)                   *
// *                                                                       *
// * This program is free software: you can redistribute it and/or modify  *
// * it under the terms of the GNU General Public License as published by  *
// * the Free Software Foundation, either version 3 of the License, or     *
// * (at your option) any later version.                                   *
// *                                                                       *
// * This program is distributed in the hope that it will be useful,       *
// * but WITHOUT ANY WARRANTY; without even the implied warranty of        *
// * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the         *
// * GNU General Public License for more details.                          *
// *                                                                       *
// * You should have received a copy of the GNU General Public License     *
// * along with this program.  If not, see <http://www.gnu.org/licenses/>. *
// *************************************************************************

use std::fs;
use std::path;

use crate::args;

use anyhow::Context as _;

/// The configuration for nitrocli, usually read from configuration
/// files and environment variables.
#[derive(Clone, Copy, Debug, Default, PartialEq, merge::Merge, serde::Deserialize)]
pub struct Config {
  /// The model to connect to.
  pub model: Option<args::DeviceModel>,
  /// Whether to bypass the cache for all secrets or not.
  #[merge(strategy = merge::bool::overwrite_false)]
  #[serde(default)]
  pub no_cache: bool,
  /// The log level.
  #[merge(strategy = merge::num::overwrite_zero)]
  #[serde(default)]
  pub verbosity: u8,
}

impl Config {
  pub fn load() -> anyhow::Result<Self> {
    use merge::Merge as _;

    let mut config = Config::default();
    if let Some(user_config) = load_user_config()? {
      config.merge(user_config);
    }
    config.merge(load_env_config()?);

    Ok(config)
  }

  pub fn update(&mut self, args: &args::Args) {
    if args.model.is_some() {
      self.model = args.model;
    }
    if args.verbose > 0 {
      self.verbosity = args.verbose;
    }
  }
}

fn load_user_config() -> anyhow::Result<Option<Config>> {
  let path = path::Path::new("config.toml");
  if path.is_file() {
    read_config_file(&path).map(Some)
  } else {
    Ok(None)
  }
}

fn load_env_config() -> anyhow::Result<Config> {
  envy::prefixed("NITROCLI_")
    .from_env()
    .context("Failed to parse environment variables")
}

pub fn read_config_file(path: &path::Path) -> anyhow::Result<Config> {
  let s = fs::read_to_string(path)
    .with_context(|| format!("Failed to read configuration file '{}'", path.display()))?;
  toml::from_str(&s)
    .with_context(|| format!("Failed to parse configuration file '{}'", path.display()))
}
