pub static ADMIN_PASSWORD: &str = "12345678";
pub static UPDATE_PIN: &str = "12345678";
pub static USER_PASSWORD: &str = "123456";

#[cfg(not(feature = "test-storage"))]
pub type Target = nitrokey::Pro;

#[cfg(feature = "test-storage")]
pub type Target = nitrokey::Storage;
