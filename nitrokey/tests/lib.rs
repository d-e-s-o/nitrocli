// Copyright (C) 2019 Robin Krahl <robin.krahl@ireas.org>
// SPDX-License-Identifier: MIT

mod util;

#[test]
fn get_library_version() {
    let version = unwrap_ok!(nitrokey::get_library_version());

    assert!(version.git.is_empty() || version.git.starts_with("v"));
    assert!(version.major > 0);
}
