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

use crate::args;
use crate::error;
use crate::Result;

/// The name of nitrocli's configuration file relative to the application configuration directory.
///
/// The application configuration directory is determined using the `directories` crate.  For Unix,
/// it is `$XDG_CONFIG_HOME/nitrocli` (defaults to `$HOME/.config/nitrocli`).
const CONFIG_FILE: &str = "config.toml";

/// The configuration for nitrocli, usually read from configuration files and environment
/// variables.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Deserialize)]
pub struct Config {
  /// The model to connect to.
  pub model: Option<args::DeviceModel>,
  /// Whether to bypass the cache for all secrets or not.
  #[serde(default)]
  pub no_cache: bool,
  #[serde(default)]
  /// The log level.
  pub verbosity: u8,
}

impl Config {
  pub fn load() -> Result<Self> {
    let project_dirs = directories::ProjectDirs::from("", "", "nitrocli")
      .ok_or_else(|| error::Error::from("Could not determine the home directory"))?;
    let config_file = project_dirs.config_dir().join(CONFIG_FILE);

    let mut config = config::Config::new();
    let _ = config
      .merge(config::File::from(config_file).format(config::FileFormat::Toml).required(false))?
      .merge(config::Environment::with_prefix("NITROCLI"))?;
    config.try_into().map_err(error::Error::from)
  }

  pub fn update(&mut self, args: &args::Args) {
    if args.model.is_some() {
      self.model = args.model;
    }
    if args.no_cache {
      self.no_cache = true;
    }
    if args.verbose > 0 {
      self.verbosity = args.verbose;
    }
  }
}
