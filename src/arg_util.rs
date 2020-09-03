// arg_util.rs

// Copyright (C) 2019-2020 The Nitrocli Developers
// SPDX-License-Identifier: GPL-3.0-or-later

macro_rules! count {
  ($head:ident) => { 1 };
  ($head:ident, $($tail:ident),*) => {
    1 + count!($($tail),*)
  }
}

/// Translate an optional source into an optional destination.
macro_rules! tr {
  ($dst:tt, $src:tt) => {
    $dst
  };
  ($dst:tt) => {};
}

macro_rules! Command {
  ( $(#[$docs:meta])* $name:ident, [
    $( $(#[$doc:meta])* $var:ident$(($inner:ty))? => $exec:expr, ) *
  ] ) => {
    $(#[$docs])*
    #[derive(Debug, PartialEq, structopt::StructOpt)]
    pub enum $name {
      $(
        $(#[$doc])*
        $var$(($inner))?,
      )*
    }

    #[allow(unused_qualifications)]
    impl $name {
      pub fn execute(
        self,
        ctx: &mut crate::Context<'_>,
      ) -> anyhow::Result<()> {
        match self {
          $(
            $name::$var$((tr!(args, $inner)))? => $exec(ctx $(,tr!(args, $inner))?),
          )*
        }
      }
    }
  };
}

/// A macro for generating an enum with a set of simple (i.e., no
/// parameters) variants and their textual representations.
// TODO: Right now we hard code the derives we create. We may want to
//       make this set configurable.
macro_rules! Enum {
  ( $(#[$docs:meta])* $name:ident, [ $( $var:ident => $str:expr, ) *] ) => {
    $(#[$docs])*
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub enum $name {
      $(
        $var,
      )*
    }

    enum_int! {$name, [
      $( $var => $str, )*
    ]}
  };
}

macro_rules! enum_int {
  ( $name:ident, [ $( $var:ident => $str:expr, ) *] ) => {
    impl $name {
      #[allow(unused)]
      pub fn all(&self) -> [$name; count!($($var),*) ] {
        $name::all_variants()
      }

      pub fn all_variants() -> [$name; count!($($var),*) ] {
        [
          $(
            $name::$var,
          )*
        ]
      }

      #[allow(unused)]
      pub fn all_str() -> [&'static str; count!($($var),*)] {
        [
          $(
            $str,
          )*
        ]
      }
    }

    impl ::std::convert::AsRef<str> for $name {
      fn as_ref(&self) -> &'static str {
        match *self {
          $(
            $name::$var => $str,
          )*
        }
      }
    }

    impl ::std::fmt::Display for $name {
      fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        write!(f, "{}", self.as_ref())
      }
    }

    impl ::std::str::FromStr for $name {
      type Err = ::std::string::String;

      fn from_str(s: &str) -> ::std::result::Result<Self, Self::Err> {
        match s {
          $(
            $str => Ok($name::$var),
          )*
          _ => Err(
            format!(
              "expected one of {}",
              $name::all_str().join(", "),
             )
           )
        }
      }
    }
  };
}

#[cfg(test)]
mod tests {
  Enum! {Command, [
    Var1 => "var1",
    Var2 => "2",
    Var3 => "crazy",
  ]}

  #[test]
  fn all_variants() {
    assert_eq!(
      Command::all_variants(),
      [Command::Var1, Command::Var2, Command::Var3]
    )
  }

  #[test]
  fn text_representations() {
    assert_eq!(Command::Var1.as_ref(), "var1");
    assert_eq!(Command::Var2.as_ref(), "2");
    assert_eq!(Command::Var3.as_ref(), "crazy");
  }
}
