pub static ADMIN_PASSWORD: &str = "12345678";
pub static USER_PASSWORD: &str = "123456";

#[cfg(feature = "test-no-device")]
pub type Target = ::Pro;

#[cfg(feature = "test-pro")]
pub type Target = ::Pro;

#[cfg(feature = "test-storage")]
pub type Target = ::Storage;
