// output.rs

// Copyright (C) 2020 The Nitrocli Developers
// SPDX-License-Identifier: GPL-3.0-or-later

//! Defines data types that can be formatted in different output formats.

use std::fmt;

use crate::args;
use crate::Context;

/// A trait for objects that can be printed as nitrocli’s output.
pub trait Output {
  /// Formats this object using the given output format.
  fn format(&self, format: args::OutputFormat) -> anyhow::Result<String>;

  /// Prints this object to the output set in the given context using the output format set in the
  /// context configuration.
  ///
  /// The default implementation for this method prints the return value of `format` to
  /// `ctx.stdout`.
  fn print(&self, ctx: &mut Context<'_>) -> anyhow::Result<()> {
    println!(
      ctx,
      "{}",
      self
        .format(ctx.config.output_format.unwrap_or(args::OutputFormat::Text))?
        .trim_end()
    )
    .map_err(From::from)
  }
}

/// A single object.
pub struct Value<T: fmt::Display>(T);

/// A list of objects of the same type that is displayed as a table with a fallback message for an
/// empty list.
pub struct Table<T: TableItem> {
  items: Vec<T>,
  empty_message: String,
}

/// A trait for objects that can be displayed in a table.
pub trait TableItem {
  /// Returns the column headers for this type of table items.
  fn headers() -> Vec<&'static str>;
  /// Returns the values of the column for this table item.
  fn values(&self) -> Vec<String>;
}

/// A helper struct for building text reprensetations of objects.
pub struct TextObject {
  name: String,
  items: Vec<(usize, String, String)>,
}

impl<T: fmt::Display> Value<T> {
  pub fn new(value: T) -> Value<T> {
    Value(value)
  }
}

impl<T: fmt::Display> Output for Value<T> {
  fn format(&self, format: args::OutputFormat) -> anyhow::Result<String> {
    match format {
      args::OutputFormat::Text => Ok(self.0.to_string()),
    }
  }
}

impl<T: TableItem> Table<T> {
  pub fn new(empty_message: impl Into<String>) -> Table<T> {
    Table {
      items: Vec::new(),
      empty_message: empty_message.into(),
    }
  }

  pub fn push(&mut self, item: T) {
    self.items.push(item);
  }

  pub fn append(&mut self, vec: &mut Vec<T>) {
    self.items.append(vec);
  }
}

impl<T: TableItem> Output for Table<T> {
  fn format(&self, format: args::OutputFormat) -> anyhow::Result<String> {
    match format {
      args::OutputFormat::Text => {
        if self.items.is_empty() {
          Ok(self.empty_message.clone())
        } else {
          let headers = T::headers().into_iter().map(ToOwned::to_owned).collect();
          let values = self.items.iter().map(TableItem::values);
          Ok(print_table(headers, values))
        }
      }
    }
  }
}

fn print_table<I>(headers: Vec<String>, iter: I) -> String
where
  I: Iterator<Item = Vec<String>>,
{
  let mut values = Vec::new();
  values.push(headers);
  values.extend(iter);
  let n = values.iter().map(Vec::len).min().unwrap_or_default();
  let lens: Vec<_> = (0..n)
    .map(|idx| {
      values
        .iter()
        .map(|v| v[idx].len())
        .max()
        .unwrap_or_default()
    })
    .collect();
  values
    .iter()
    .map(|v| print_table_line(&lens, &v))
    .collect::<Vec<_>>()
    .join("\n")
}

fn print_table_line(lens: &[usize], values: &[String]) -> String {
  lens
    .iter()
    .zip(values)
    .map(|(width, value)| format!("{:width$}", value, width = width))
    .collect::<Vec<_>>()
    .join("\t")
}

impl TextObject {
  pub fn new(name: impl Into<String>) -> TextObject {
    TextObject {
      name: name.into(),
      items: Vec::new(),
    }
  }

  pub fn push_line(&mut self, key: impl Into<String>, value: impl Into<String>) {
    self.items.push((1, key.into(), value.into()));
  }

  pub fn push_object(&mut self, o: TextObject) {
    self.push_line(o.name, "");
    for (indent, key, value) in o.items {
      self.items.push((1 + indent, key, value));
    }
  }
}

impl fmt::Display for TextObject {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    writeln!(f, "{}:", self.name)?;
    let max_len = self
      .items
      .iter()
      .map(|(indent, key, _)| indent * 2 + key.len())
      .max()
      .unwrap_or(0);
    for (indent, key, value) in &self.items {
      let prefix = " ".repeat(indent * 2);
      let padding = " ".repeat(max_len - key.len() - indent * 2);
      writeln!(f, "{}{}:{} {}", prefix, key, padding, value)?;
    }
    Ok(())
  }
}
