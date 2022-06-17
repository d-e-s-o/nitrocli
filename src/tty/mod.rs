// mod.rs

// Copyright (C) 2022 The Nitrocli Developers
// SPDX-License-Identifier: GPL-3.0-or-later

#[cfg(target_os = "linux")]
mod linux;
#[cfg(not(target_os = "linux"))]
mod stub;

#[cfg(target_os = "linux")]
pub(crate) use linux::retrieve_tty;
#[cfg(not(target_os = "linux"))]
pub(crate) use stub::retrieve_tty;
