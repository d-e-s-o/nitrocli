// output.rs

// Copyright (C) 2020 The Nitrocli Developers
// SPDX-License-Identifier: GPL-3.0-or-later

//! Defines data types that can be formatted in different output formats.

use std::collections;
use std::fmt;

use anyhow::Context as _;

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
pub struct Value<T: fmt::Display + serde::Serialize> {
  key: String,
  value: T,
}

/// A list of objects of the same type that is displayed as a table with a fallback message for an
/// empty list.
pub struct Table<T: TableItem> {
  key: String,
  items: Vec<T>,
  empty_message: String,
}

/// A trait for objects that can be displayed in a table.
pub trait TableItem: serde::Serialize {
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

impl<T: fmt::Display + serde::Serialize> Value<T> {
  pub fn new(key: impl Into<String>, value: T) -> Value<T> {
    Value {
      key: key.into(),
      value,
    }
  }
}

impl<T: fmt::Display + serde::Serialize> Output for Value<T> {
  fn format(&self, format: args::OutputFormat) -> anyhow::Result<String> {
    match format {
      args::OutputFormat::Json => get_json(&self.key, &self.value),
      args::OutputFormat::Tsv => get_tsv_object(&self.value),
      args::OutputFormat::Text => Ok(self.value.to_string()),
    }
  }
}

impl<T: TableItem> Table<T> {
  pub fn new(key: impl Into<String>, empty_message: impl Into<String>) -> Table<T> {
    Table {
      key: key.into(),
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
      args::OutputFormat::Json => get_json(&self.key, &self.items),
      args::OutputFormat::Tsv => get_tsv_list(&self.items),
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

fn get_json<T: serde::Serialize + ?Sized>(key: &str, value: &T) -> anyhow::Result<String> {
  let mut map = collections::HashMap::new();
  let _ = map.insert(key, value);
  serde_json::to_string_pretty(&map).context("Could not serialize output to JSON")
}

fn get_tsv_list<T: serde::Serialize>(items: &[T]) -> anyhow::Result<String> {
  let mut writer = csv::WriterBuilder::new()
    .delimiter(b'\t')
    .from_writer(vec![]);
  for item in items {
    writer
      .serialize(item)
      .context("Could not serialize output to TSV")?;
  }
  String::from_utf8(writer.into_inner()?).context("Could not parse TSV output as UTF-8")
}

fn get_tsv_object<T: serde::Serialize>(value: T) -> anyhow::Result<String> {
  let value = serde_json::to_value(&value).context("Could not serialize output")?;
  get_tsv_list(&get_tsv_records(&[], value))
}

/// Converts an arbitrary value into a list of TSV records.
///
/// There are two cases:  Scalars are converted to a single value (without headers).  Arrays and
/// objects are converted to a list of key-value pairs (with headers).  Nested arrays and objects
/// are flattened and their keys are separated by dots.
///
/// `prefix` is the prefix to use for the keys of arrays and objects, or an empty slice if the
/// given value is the top-level value.
fn get_tsv_records(prefix: &[&str], value: serde_json::Value) -> Vec<serde_json::Value> {
  use serde_json::Value;

  let mut vec = Vec::new();
  if (value.is_array() || value.is_object()) && prefix.is_empty() {
    vec.push(Value::Array(vec!["key".into(), "value".into()]));
  }

  match value {
    Value::Object(o) => {
      for (key, value) in o {
        vec.append(&mut get_tsv_records(&[prefix, &[&key]].concat(), value));
      }
    }
    Value::Array(a) => {
      for (idx, value) in a.into_iter().enumerate() {
        let idx = idx.to_string();
        vec.append(&mut get_tsv_records(&[prefix, &[&idx]].concat(), value));
      }
    }
    _ => {
      if prefix.is_empty() {
        vec.push(value);
      } else {
        vec.push(Value::Array(vec![prefix.join(".").into(), value]));
      }
    }
  }

  vec
}
