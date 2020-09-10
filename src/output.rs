// commands.rs

// Copyright (C) 2020 The Nitrocli Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::Context;

/// A progress bar that can be printed to an interactive output.
pub struct ProgressBar {
  redraw: bool,
  progress: u8,
  toggle: bool,
  finished: bool,
}

impl ProgressBar {
  pub fn new() -> ProgressBar {
    ProgressBar {
      redraw: true,
      progress: 0,
      toggle: false,
      finished: false,
    }
  }

  pub fn is_finished(&self) -> bool {
    self.finished
  }

  pub fn update(&mut self, progress: u8) -> anyhow::Result<()> {
    anyhow::ensure!(!self.finished, "Tried to update finished progress bar");
    anyhow::ensure!(
      progress <= 100,
      "Progress bar value out of range: {}",
      progress
    );
    if progress != self.progress {
      self.redraw = true;
      self.progress = progress;
    }
    self.toggle = !self.toggle;
    Ok(())
  }

  pub fn finish(&mut self) {
    self.finished = true;
    self.redraw = true;
    self.progress = 100;
  }

  pub fn draw(&self, ctx: &mut Context<'_>) -> anyhow::Result<()> {
    use crossterm::{cursor, terminal};

    if !ctx.is_tty {
      return Ok(());
    }

    let progress_char = if self.toggle && !self.finished {
      "."
    } else {
      " "
    };
    if self.redraw {
      use progressing::Baring;

      let mut progress_bar = progressing::mapping::Bar::with_range(0, 100);
      progress_bar.set(self.progress);

      print!(ctx, "{}", terminal::Clear(terminal::ClearType::CurrentLine))?;
      print!(ctx, "{}", cursor::MoveToColumn(0))?;
      print!(ctx, " {} {}", progress_char, progress_bar)?;
      if self.finished {
        println!(ctx)?;
      }
    } else {
      print!(ctx, "{}{}", cursor::MoveToColumn(1), progress_char)?;
    }

    ctx.stdout.flush()?;
    Ok(())
  }
}
