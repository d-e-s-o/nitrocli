// mod.rs

// Copyright (C) 2019-2021 The Nitrocli Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use std::ffi;

use nitrokey_test::test as test_device;

mod config;
mod encrypted;
mod extensions;
mod fill;
mod hidden;
mod list;
mod lock;
mod otp;
mod pin;
mod pws;
mod reset;
mod run;
mod status;
mod unencrypted;

struct Nitrocli {
  stdin: String,
  model: Option<nitrokey::Model>,
  path: Option<ffi::OsString>,
  admin_pin: Option<ffi::OsString>,
  user_pin: Option<ffi::OsString>,
  new_admin_pin: Option<ffi::OsString>,
  new_user_pin: Option<ffi::OsString>,
  password: Option<ffi::OsString>,
}

impl Nitrocli {
  pub fn new() -> Self {
    Self {
      stdin: String::new(),
      model: None,
      path: None,
      admin_pin: Some(nitrokey::DEFAULT_ADMIN_PIN.into()),
      user_pin: Some(nitrokey::DEFAULT_USER_PIN.into()),
      new_admin_pin: None,
      new_user_pin: None,
      password: None,
    }
  }

  /// Set the model to use.
  fn model(mut self, model: nitrokey::Model) -> Self {
    self.model = Some(model);
    self
  }

  /// Set the password to use for certain operations.
  fn password(mut self, password: impl Into<ffi::OsString>) -> Self {
    self.password = Some(password.into());
    self
  }

  /// Set the `PATH` used for looking up extensions.
  fn path(mut self, path: impl Into<ffi::OsString>) -> Self {
    self.path = Some(path.into());
    self
  }

  pub fn stdin(mut self, stdin: impl Into<String>) -> Self {
    self.stdin = stdin.into();
    self
  }

  pub fn admin_pin(mut self, pin: impl Into<ffi::OsString>) -> Self {
    self.admin_pin = Some(pin.into());
    self
  }

  pub fn new_admin_pin(mut self, pin: impl Into<ffi::OsString>) -> Self {
    self.new_admin_pin = Some(pin.into());
    self
  }

  pub fn user_pin(mut self, pin: impl Into<ffi::OsString>) -> Self {
    self.user_pin = Some(pin.into());
    self
  }

  pub fn new_user_pin(mut self, pin: impl Into<ffi::OsString>) -> Self {
    self.new_user_pin = Some(pin.into());
    self
  }

  fn model_to_arg(model: nitrokey::Model) -> &'static str {
    match model {
      nitrokey::Model::Librem => "--model=librem",
      nitrokey::Model::Pro => "--model=pro",
      nitrokey::Model::Storage => "--model=storage",
      _ => panic!("Unexpected model in test suite: {}", model),
    }
  }

  fn do_run<F, R>(&mut self, args: &[&str], f: F) -> (R, Vec<u8>, Vec<u8>)
  where
    F: FnOnce(&mut crate::Context<'_>, Vec<String>) -> R,
  {
    let args = ["nitrocli"]
      .iter()
      .cloned()
      .chain(self.model.map(Self::model_to_arg))
      .chain(args.iter().cloned())
      .map(ToOwned::to_owned)
      .collect();

    let mut stdin = self.stdin.as_bytes();
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();

    let ctx = &mut crate::Context {
      stdin: &mut stdin,
      stdout: &mut stdout,
      stderr: &mut stderr,
      is_tty: false,
      path: self.path.clone(),
      admin_pin: self.admin_pin.clone(),
      user_pin: self.user_pin.clone(),
      new_admin_pin: self.new_admin_pin.clone(),
      new_user_pin: self.new_user_pin.clone(),
      password: self.password.clone(),
      config: crate::config::Config {
        no_cache: true,
        ..Default::default()
      },
    };

    (f(ctx, args), stdout, stderr)
  }

  /// Run `nitrocli`'s `run` function.
  pub fn run(&mut self, args: &[&str]) -> (i32, Vec<u8>, Vec<u8>) {
    self.do_run(args, |c, a| crate::run(c, a))
  }

  /// Run `nitrocli`'s `handle_arguments` function.
  pub fn handle(&mut self, args: &[&str]) -> anyhow::Result<String> {
    let (res, out, _) = self.do_run(args, crate::handle_arguments);
    res.map(|_| String::from_utf8_lossy(&out).into_owned())
  }
}
