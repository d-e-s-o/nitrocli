// commands.rs

// Copyright (C) 2020 The Nitrocli Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::Context;

/// A progress bar that can be printed to an interactive output.
pub struct ProgressBar {
  /// Whether to redraw the entire progress bar in the next call to `draw`.
  redraw: bool,
  /// The current progress of the progress bar (0 <= progress <= 100).
  progress: u8,
  /// Toogled on every call to `draw` to print a pulsing indicator.
  toggle: bool,
  /// Whether this progress bar finished.
  finished: bool,
}

impl ProgressBar {
  /// Creates a new empty progress bar.
  pub fn new() -> ProgressBar {
    ProgressBar {
      redraw: true,
      progress: 0,
      toggle: false,
      finished: false,
    }
  }

  /// Whether this progress bar is finished.
  pub fn is_finished(&self) -> bool {
    self.finished
  }

  /// Updates the progress bar with the given progress (0 <= progress <= 100).
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

  /// Finish this progress bar.
  ///
  /// A finished progress bar may no longer be updated.
  pub fn finish(&mut self) {
    self.finished = true;
    self.redraw = true;
    self.progress = 100;
  }

  /// Print the progress bar to the stdout set in the given context.
  ///
  /// On every call of this method (as long as the progress bar is not finished), a pulsing
  /// indicator is printed to show that the process is still running.  If there was progress since
  /// the last call to `draw`, or if this is the first call, this function will also print the
  /// progress bar itself.
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
