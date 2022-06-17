// stub.rs

// Copyright (C) 2022 The Nitrocli Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use std::ffi;

pub(crate) fn retrieve_tty() -> Result<ffi::OsString, ()> {
  Err(())
}
