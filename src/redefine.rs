// redefine.rs

// Copyright (C) 2019-2020 The Nitrocli Developers
// SPDX-License-Identifier: GPL-3.0-or-later

// A replacement of the standard println!() macro that requires an
// execution context as the first argument and prints to its stdout.
macro_rules! println {
  ($ctx:expr) => {
    writeln!($ctx.stdout, "")
  };
  ($ctx:expr, $($arg:tt)*) => {
    writeln!($ctx.stdout, $($arg)*)
  };
}

macro_rules! print {
  ($ctx:expr, $($arg:tt)*) => {
    write!($ctx.stdout, $($arg)*)
  };
}
