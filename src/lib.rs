//! Simple concurrent progress bars.

use std::io::{Stdout, Write};

// - Replicate ILoveCandy
// - No (or few) dependencies
// - Spinners...?

// Terminal Support:
//
// - [x] Alacritty
// - [x] xterm
// - [x] Linux console

/// A progress bar "coordinator" to share between threads.
pub struct Progress {
    bars: Vec<SubBar>,
    stdout: Stdout,
}

impl Progress {
    /// Initialize a new progress bar coordinator.
    pub fn new() -> Progress {
        let stdout = std::io::stdout();
        let bars = vec![];
        Progress { bars, stdout }
    }

    /// Like [`Progress::new`] but accepts a size hint to avoid reallocation as bar count grows.
    pub fn with_capacity(capacity: usize) -> Progress {
        let stdout = std::io::stdout();
        let bars = Vec::with_capacity(capacity);
        Progress { bars, stdout }
    }

    /// Create a new progress bar with default styling and receive an owned
    /// handle to it.
    pub fn bar(&mut self, total: usize) -> Bar {
        // let prev = 0;
        let curr = 0;
        let bar = SubBar { curr, total };
        self.bars.push(bar);
        println!();
        Bar(self.bars.len() - 1)
    }

    /// Set a particular [`Bar`]'s progress value, but don't draw it.
    pub fn set(&mut self, bar: &Bar, value: usize) {
        self.bars[bar.0].curr = value;
    }

    /// Force the drawing of a particular [`Bar`].
    ///
    /// **Note:** Drawing will only occur if there is something to show. Namely,
    /// if the progress bar should advance by at least one visible "tick".
    pub fn draw(&mut self, bar: &Bar) {
        let b = &self.bars[bar.0];
        let pos = self.bars.len() - bar.0;

        if b.curr >= b.total {
            print!(
                "\x1B[s\x1B[{}A\r{:02} [{:#>f$}]\x1B[u\r",
                pos,
                bar.0,
                "",
                f = 50
            )
        } else {
            let f = (50 * b.curr / b.total).min(49);
            let e = 49 - f;
            let pos = self.bars.len() - bar.0;

            print!(
                "\x1B[s\x1B[{}A\r{:02} [{:#>f$}{}{:->e$}]\x1B[u\r",
                pos,
                bar.0,
                "",
                '>',
                "",
                f = f,
                e = e
            );
        }

        // Very important, or the output won't appear fluid.
        self.stdout.flush().unwrap();
    }

    /// Set a [`Bar`]'s value and immediately try to draw it.
    pub fn set_and_draw(&mut self, bar: &Bar, value: usize) {
        self.set(bar, value);
        self.draw(bar);
    }

    /// Increment a given [`Bar`]'s progress, but don't draw it.
    pub fn inc(&mut self, bar: &Bar, value: usize) {
        self.set(bar, self.bars[bar.0].curr + value)
    }

    /// Increment a given [`Bar`]'s progress and immediately try to draw it.
    pub fn inc_and_draw(&mut self, bar: &Bar, value: usize) {
        self.inc(bar, value);
        self.draw(bar);
    }

    /// Has the given bar completed?
    pub fn is_done(&self, bar: &Bar) -> bool {
        let b = &self.bars[bar.0];
        b.curr >= b.total
    }
}

struct SubBar {
    // prev: usize,
    curr: usize,
    total: usize,
}

/// A progress bar index for use with [`Progress`].
///
/// This type has no meaningful methods of its own. Individual bars are advanced
/// by method calls on `Progress`:
///
/// ```
/// use linya::Progress;
///
/// let mut progress = Progress::new();
/// let bar = progress.bar(100);
/// progress.inc_and_draw(&bar, 1);
/// ```
pub struct Bar(usize);
