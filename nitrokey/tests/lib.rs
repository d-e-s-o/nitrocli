// Copyright (C) 2019 Robin Krahl <robin.krahl@ireas.org>
// SPDX-License-Identifier: MIT

mod util;

#[test]
fn get_library_version() {
    let version = unwrap_ok!(nitrokey::get_library_version());

    assert!(version.git.is_empty() || version.git.starts_with("v"));
    assert!(version.major > 0);
}

#[test]
fn take_manager() {
    assert!(nitrokey::take().is_ok());

    let result = nitrokey::take();
    assert!(result.is_ok());
    let result2 = nitrokey::take();
    match result2 {
        Ok(_) => panic!("Expected error, got Ok(_)!"),
        Err(nitrokey::Error::ConcurrentAccessError) => {}
        Err(err) => panic!("Expected ConcurrentAccessError, got {}", err),
    }
    drop(result);
    assert!(nitrokey::take().is_ok());
}
