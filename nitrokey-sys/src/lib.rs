#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

mod ffi;

pub use crate::ffi::*;

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn login_auto() {
        unsafe {
            assert_eq!(0, NK_login_auto());
        }
    }

    #[test]
    fn login() {
        unsafe {
            // Unconnected
            assert_eq!(0, NK_login(CString::new("S").unwrap().as_ptr()));
            assert_eq!(0, NK_login(CString::new("P").unwrap().as_ptr()));
            // Unsupported model
            assert_eq!(0, NK_login(CString::new("T").unwrap().as_ptr()));
        }
    }
}
