// error.rs

// *************************************************************************
// * Copyright (C) 2017-2019 Daniel Mueller (deso@posteo.net)              *
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
use std::str;
use std::string;

use structopt::clap;

/// A trait used to simplify error handling in conjunction with the
/// try_with_* functions we use for repeatedly asking the user for a
/// secret.
pub trait TryInto<T> {
  fn try_into(self) -> Result<T, Error>;
}

impl<T, U> TryInto<U> for T
where
  T: Into<U>,
{
  fn try_into(self) -> Result<U, Error> {
    Ok(self.into())
  }
}

#[derive(Debug)]
pub enum Error {
  ClapError(clap::Error),
  IoError(io::Error),
  NitrokeyError(Option<&'static str>, nitrokey::Error),
  Utf8Error(str::Utf8Error),
  Error(String),
}

impl TryInto<nitrokey::Error> for Error {
  fn try_into(self) -> Result<nitrokey::Error, Error> {
    match self {
      Error::NitrokeyError(_, err) => Ok(err),
      err => Err(err),
    }
  }
}

impl From<&str> for Error {
  fn from(s: &str) -> Error {
    Error::Error(s.to_string())
  }
}

impl From<clap::Error> for Error {
  fn from(e: clap::Error) -> Error {
    Error::ClapError(e)
  }
}

impl From<nitrokey::Error> for Error {
  fn from(e: nitrokey::Error) -> Error {
    Error::NitrokeyError(None, e)
  }
}

impl From<io::Error> for Error {
  fn from(e: io::Error) -> Error {
    Error::IoError(e)
  }
}

impl From<str::Utf8Error> for Error {
  fn from(e: str::Utf8Error) -> Error {
    Error::Utf8Error(e)
  }
}

impl From<string::FromUtf8Error> for Error {
  fn from(e: string::FromUtf8Error) -> Error {
    Error::Utf8Error(e.utf8_error())
  }
}

impl fmt::Display for Error {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match *self {
      Error::ClapError(ref e) => write!(f, "{}", e),
      Error::NitrokeyError(ref ctx, ref e) => {
        if let Some(ctx) = ctx {
          write!(f, "{}: ", ctx)?;
        }
        write!(f, "{}", e)
      }
      Error::Utf8Error(_) => write!(f, "Encountered UTF-8 conversion error"),
      Error::IoError(ref e) => write!(f, "IO error: {}", e),
      Error::Error(ref e) => write!(f, "{}", e),
    }
  }
}
