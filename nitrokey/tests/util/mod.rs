// Copyright (C) 2018-2019 Robin Krahl <robin.krahl@ireas.org>
// SPDX-License-Identifier: MIT

#[macro_export]
macro_rules! unwrap_ok {
    ($val:expr) => {{
        match $val {
            Ok(val) => val,
            Err(err) => panic!(
                r#"assertion failed: `(left == right)`
  left: `Ok(_)`,
 right: `Err({:?})`"#,
                err
            ),
        }
    }};
}

#[macro_export]
macro_rules! assert_any_ok {
    ($val:expr) => {{
        match &$val {
            Ok(_) => {}
            Err(err) => panic!(
                r#"assertion failed: `(left == right)`
  left: `Ok(_)`,
 right: `Err({:?})`"#,
                err
            ),
        }
    }};
}

#[macro_export]
macro_rules! assert_ok {
    ($left:expr, $right:expr) => {{
        match &$right {
            Ok(right) => match &$left {
                left => {
                    if !(*left == *right) {
                        panic!(
                            r#"assertion failed: `(left == right)`
  left: `{:?}`,
 right: `{:?}`"#,
                            left, right
                        )
                    }
                }
            },
            Err(right_err) => panic!(
                r#"assertion failed: `(left == right)`
  left: `Ok({:?})`,
 right: `Err({:?})`"#,
                $left, right_err
            ),
        }
    }};
}

#[macro_export]
macro_rules! assert_err {
    ($err:path, $left:expr, $right:expr) => {
        match &$right {
            Err($err(ref right_err)) => match &$left {
                left_err => {
                    if !(*left_err == *right_err) {
                        panic!(
                            r#"assertion failed: `(left == right)`
  left: `{:?}`,
 right: `{:?}`"#,
                            left_err, right_err
                        )
                    }
                }
            },
            Err(ref right_err) => panic!(
                r#"assertion failed: `(left == right)`
  left: `{:?}`,
 right: `{:?}`"#,
                $err($left),
                right_err
            ),
            Ok(right_ok) => panic!(
                r#"assertion failed: `(left == right)`
  left: `Err({:?})`,
 right: `Ok({:?})`"#,
                $err($left),
                right_ok
            ),
        }
    };
}

#[macro_export]
macro_rules! assert_cmd_err {
    ($left:expr, $right:expr) => {
        assert_err!(::nitrokey::Error::CommandError, $left, $right);
    };
}

#[macro_export]
macro_rules! assert_cmu_err {
    ($left:expr, $right:expr) => {
        assert_err!(::nitrokey::Error::CommunicationError, $left, $right);
    };
}

#[macro_export]
macro_rules! assert_lib_err {
    ($left:expr, $right:expr) => {
        assert_err!(::nitrokey::Error::LibraryError, $left, $right);
    };
}
