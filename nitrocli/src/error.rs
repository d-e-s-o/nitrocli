// error.rs

// *************************************************************************
// * Copyright (C) 2017-2018 Daniel Mueller (deso@posteo.net)              *
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

use std::fmt;
use std::io;
use std::string;

#[derive(Debug)]
pub enum Error {
  ArgparseError(i32),
  IoError(io::Error),
  Utf8Error(string::FromUtf8Error),
  Error(String),
}

impl From<io::Error> for Error {
  fn from(e: io::Error) -> Error {
    Error::IoError(e)
  }
}

impl From<string::FromUtf8Error> for Error {
  fn from(e: string::FromUtf8Error) -> Error {
    Error::Utf8Error(e)
  }
}

impl fmt::Display for Error {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match *self {
      Error::ArgparseError(_) => write!(f, "Could not parse arguments"),
      Error::Utf8Error(_) => write!(f, "Encountered UTF-8 conversion error"),
      Error::IoError(ref e) => write!(f, "IO error: {}", e.get_ref().unwrap()),
      Error::Error(ref e) => write!(f, "{}", e),
    }
  }
}
