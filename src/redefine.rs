// redefine.rs

// *************************************************************************
// * Copyright (C) 2019 The Nitrocli Developers                            *
// *                                                                       *
// * This program is free software: you can redistribute it and/or modify  *
// * it under the terms of the GNU General Public License as published by  *
// * the Free Software Foundation, either version 3 of the License, or     *
// * (at your option) any later version.                                   *
// *                                                                       *
// * This program is distributed in the hope that it will be useful,       *
// * but WITHOUT ANY WARRANTY; without even the implied warranty of        *
// * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the         *
// * GNU General Public License for more details.                          *
// *                                                                       *
// * You should have received a copy of the GNU General Public License     *
// * along with this program.  If not, see <http://www.gnu.org/licenses/>. *
// *************************************************************************

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

macro_rules! eprintln {
  ($ctx:expr) => {
    writeln!($ctx.stderr, "")
  };
  ($ctx:expr, $($arg:tt)*) => {
    writeln!($ctx.stderr, $($arg)*)
  };
}
