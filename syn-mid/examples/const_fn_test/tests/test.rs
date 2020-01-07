#![cfg_attr(nightly, feature(const_fn, const_vec_new))]
#![warn(rust_2018_idioms)]
#![allow(dead_code)]

use const_fn::const_fn;

#[const_fn(nightly)]
fn const_vec_new<T>() -> Vec<T> {
    let vec = Vec::new();
    vec
}

#[test]
fn test_stable() {
    assert_eq!(const_vec_new::<u8>(), Vec::new());
}

#[cfg(nightly)]
const CONST_UNSTABLE: Vec<u8> = const_vec_new();

#[cfg(nightly)]
#[test]
fn test_unstable() {
    assert_eq!(CONST_UNSTABLE, Vec::new());
}
