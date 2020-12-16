//! Simple concurrent progress bars.
//!
//! # Features
//!
//! - Intuitive API.
//! - First-class support for `rayon`, etc.
//! - Efficient redraws.
//! - Addition of new subbars on-the-fly.
//! - Single-threaded multi-bars.
//! - Light-weight / no dependencies.
//!
//! # Usage
//!
//! `linya` is designed around the multi-bar case, and unlike other progress bar
//! libraries, has no separate type for individual bars. Instead, we use the
//! [`Progress`] type, a "bar coordinator".
//!
//! ## Multi Bars
//!
//! `Progress` does not implement [`Clone`], [`Send`], or [`Sync`], and so must
//! be wrapped in the usual [concurrent sharing types][arcmutex] before being
//! passed between threads:
//!
//! ```
//! use std::sync::{Arc, Mutex};
//! use linya::{Bar, Progress};
//! use rayon::prelude::*;
//!
//! let progress = Arc::new(Mutex::new(Progress::new()));
//!
//! // `into_par_iter()` is from `rayon`, and lets us parallelize some
//! // operation over a collection "for free".
//! (0..10).into_par_iter().for_each_with(progress, |p, n| {
//!   let bar: Bar = p.lock().unwrap().bar(50);
//!
//!   // ... Your logic ...
//!
//!   // Increment the bar and draw it immediately.
//!   // This is likely called in some inner loop or other closure.
//!   p.lock().unwrap().inc_and_draw(&bar, 10);
//! });
//! ```
//!
//! Notice that new bars are added on-the-fly from within forked threads. We
//! call [`Progress::bar`] to obtain a new "bar handle", and then pass that
//! handle back to the parent `Progress` when incrementing/drawing.
//!
//! See [`Progress::inc_and_draw`] and [`Progress::set_and_draw`] to advance and
//! render the bars.
//!
//! ## Single Bars
//!
//! `Progress` can also be used in a single-threaded context for individual
//! bars. The usage is the same, except that no locking is required:
//!
//! ```
//! use linya::{Bar, Progress};
//!
//! let mut progress = Progress::new();
//! let bar: Bar = progress.bar(50);
//!
//! // Use in a loop, etc.
//! progress.set_and_draw(&bar, 10);
//! ```
//!
//! In this way, you could even have multi-bars in a single-threaded context.
//!
//! ## Styling
//!
//! # Caveats
//!
//! - Your terminal must support ANSI codes.
//! - No dedicated render thread, to keep usage simple.
//! - No bar templating, to avoid dependencies.
//! - No "rates", since rerenders are not time-based.
//! - No spinners, also due to no sense of time.
//!
//! # Trivia
//!
//! *Linya* is the Quenya word for "pool", as in a [beautiful mountain pool][mirrormere].
//!
//! [mirrormere]: https://www.tednasmith.com/tolkien/durins-crown-and-the-mirrormere/
//! [arcmutex]: https://doc.rust-lang.org/stable/book/ch16-03-shared-state.html?#atomic-reference-counting-with-arct

use std::io::{Stdout, Write};

// - Replicate ILoveCandy
// - No (or few) dependencies
// - Spinners...?
// - Show example usage with `curl`

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
    ///
    /// # Panics
    ///
    /// Passing `0` to this function will cause a panic the first time a draw is
    /// attempted.
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
///
/// As shown above, this type can only be constructed via [`Progress::bar`].
pub struct Bar(usize);
