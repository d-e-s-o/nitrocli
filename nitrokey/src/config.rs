use crate::util::CommandError;

/// The configuration for a Nitrokey.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Config {
    /// If set, the stick will generate a code from the HOTP slot with the given number if numlock
    /// is pressed.  The slot number must be 0, 1 or 2.
    pub numlock: Option<u8>,
    /// If set, the stick will generate a code from the HOTP slot with the given number if capslock
    /// is pressed.  The slot number must be 0, 1 or 2.
    pub capslock: Option<u8>,
    /// If set, the stick will generate a code from the HOTP slot with the given number if
    /// scrollock is pressed.  The slot number must be 0, 1 or 2.
    pub scrollock: Option<u8>,
    /// If set, OTP generation using [`get_hotp_code`][] or [`get_totp_code`][] requires user
    /// authentication.  Otherwise, OTPs can be generated without authentication.
    ///
    /// [`get_hotp_code`]: trait.ProvideOtp.html#method.get_hotp_code
    /// [`get_totp_code`]: trait.ProvideOtp.html#method.get_totp_code
    pub user_password: bool,
}

#[derive(Debug)]
pub struct RawConfig {
    pub numlock: u8,
    pub capslock: u8,
    pub scrollock: u8,
    pub user_password: bool,
}

fn config_otp_slot_to_option(value: u8) -> Option<u8> {
    if value < 3 {
        return Some(value);
    }
    None
}

fn option_to_config_otp_slot(value: Option<u8>) -> Result<u8, CommandError> {
    match value {
        Some(value) => {
            if value < 3 {
                Ok(value)
            } else {
                Err(CommandError::InvalidSlot)
            }
        }
        None => Ok(255),
    }
}

impl Config {
    /// Constructs a new instance of this struct.
    pub fn new(
        numlock: Option<u8>,
        capslock: Option<u8>,
        scrollock: Option<u8>,
        user_password: bool,
    ) -> Config {
        Config {
            numlock,
            capslock,
            scrollock,
            user_password,
        }
    }
}

impl RawConfig {
    pub fn try_from(config: Config) -> Result<RawConfig, CommandError> {
        Ok(RawConfig {
            numlock: option_to_config_otp_slot(config.numlock)?,
            capslock: option_to_config_otp_slot(config.capslock)?,
            scrollock: option_to_config_otp_slot(config.scrollock)?,
            user_password: config.user_password,
        })
    }
}

impl From<[u8; 5]> for RawConfig {
    fn from(data: [u8; 5]) -> Self {
        RawConfig {
            numlock: data[0],
            capslock: data[1],
            scrollock: data[2],
            user_password: data[3] != 0,
        }
    }
}

impl Into<Config> for RawConfig {
    fn into(self) -> Config {
        Config {
            numlock: config_otp_slot_to_option(self.numlock),
            capslock: config_otp_slot_to_option(self.capslock),
            scrollock: config_otp_slot_to_option(self.scrollock),
            user_password: self.user_password,
        }
    }
}
