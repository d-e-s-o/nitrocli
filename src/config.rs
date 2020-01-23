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

use std::str::FromStr;

use crate::args;
use crate::Result;

/// The configuration for nitrocli, usually read from configuration files and environment
/// variables.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Config {
  /// The model to connect to.
  pub model: Option<args::DeviceModel>,
  /// Whether to bypass the cache for all secrets or not.
  pub no_cache: bool,
  /// The log level.
  pub verbosity: u8,
}

impl Config {
  pub fn load() -> Result<Self> {
    let mut config = config::Config::new();
    let _ = config.set_default("model", "")?;
    let _ = config.set_default("no_cache", false)?;
    let _ = config.set_default("verbosity", 0)?;
    let _ = config.merge(get_config_file("config.toml"))?;
    Ok(Self {
      model: args::DeviceModel::from_str(&config.get_str("model")?).ok(),
      no_cache: config.get_bool("no_cache")?,
      verbosity: config.get_int("verbosity")? as u8,
      ..Default::default()
    })
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

fn get_config_file(name: &str) -> config::File<config::FileSourceFile> {
  config::File::with_name(name)
    .format(config::FileFormat::Toml)
    .required(false)
}
