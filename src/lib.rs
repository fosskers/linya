//! Simple concurrent progress bars.
//!
//! # Features
//!
//! - Intuitive API.
//! - First-class support for `rayon`, etc.
//! - Efficient, no-allocation redraws.
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
//!   let bar: Bar = p.lock().unwrap().bar(50, format!("Downloading {}", n));
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
//! let bar: Bar = progress.bar(50, "Downloading");
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
//! - No dynamic resizing of bars if window size changes.
//!
//! # Trivia
//!
//! *Linya* is the Quenya word for "pool", as in a [beautiful mountain pool][mirrormere].
//!
//! [mirrormere]: https://www.tednasmith.com/tolkien/durins-crown-and-the-mirrormere/
//! [arcmutex]: https://doc.rust-lang.org/stable/book/ch16-03-shared-state.html?#atomic-reference-counting-with-arct

use std::io::{Stdout, Write};
use terminal_size::{terminal_size, Width};

// - Replicate ILoveCandy
// - Show example usage with `curl`

/// A progress bar "coordinator" to share between threads.
pub struct Progress {
    /// The drawable bars themselves.
    bars: Vec<SubBar>,
    /// A shared handle to `Stdout`, for buffer flushing.
    out: Stdout,
    /// Terminal width.
    width: Option<usize>,
}

impl Progress {
    /// Initialize a new progress bar coordinator.
    pub fn new() -> Progress {
        let out = std::io::stdout();
        let bars = vec![];
        let width = terminal_size().map(|(Width(w), _)| w as usize);
        Progress { bars, out, width }
    }

    /// Like [`Progress::new`] but accepts a size hint to avoid reallocation as bar count grows.
    pub fn with_capacity(capacity: usize) -> Progress {
        let out = std::io::stdout();
        let bars = Vec::with_capacity(capacity);
        let width = terminal_size().map(|(Width(w), _)| w as usize);
        Progress { bars, out, width }
    }

    /// Create a new progress bar with default styling and receive an owned
    /// handle to it.
    ///
    /// # Panics
    ///
    /// Passing `0` to this function will cause a panic the first time a draw is
    /// attempted.
    pub fn bar<'a, S: Into<String>>(&mut self, total: usize, lbl: S) -> Bar {
        let width = self.width.unwrap_or(100) / 2;
        let label = lbl.into();

        // An initial "empty" rendering of the new bar.
        println!("{:<l$} [{:->f$}]   0%", label, l = width - 9, f = width);

        // let prev = 0;
        let curr = 0;
        let bar = SubBar { curr, total, label };
        self.bars.push(bar);
        Bar(self.bars.len() - 1)
    }

    /// Set a particular [`Bar`]'s progress value, but don't draw it.
    pub fn set(&mut self, bar: &Bar, value: usize) {
        self.bars[bar.0].curr = value;
    }

    /// Force the drawing of a particular [`Bar`].
    ///
    /// **Note 1:** Drawing will only occur if there is something to show. Namely,
    /// if the progress bar should advance by at least one visible "tick".
    ///
    /// **Note 2:** If your program is not being run in a terminal, an initial
    /// empty bar will be printed but never refreshed.
    pub fn draw(&mut self, bar: &Bar) {
        // If there is no legal width value present, that means we aren't
        // running in a terminal, and no rerendering can be done.
        if let Some(term_width) = self.width {
            let b = &self.bars[bar.0];
            let pos = self.bars.len() - bar.0;
            let w = (term_width / 2) - 7;

            if b.curr >= b.total {
                print!(
                    "\x1B[s\x1B[{}A\r{:<l$} [{:#>f$}] 100%\x1B[u\r",
                    pos,
                    b.label,
                    "",
                    l = term_width - w - 8,
                    f = w,
                )
            } else {
                let f = (w * b.curr / b.total).min(w - 1);
                let e = (w - 1) - f;
                let pos = self.bars.len() - bar.0;

                print!(
                    "\x1B[s\x1B[{}A\r{:<l$} [{:#>f$}{}{:->e$}] {:3}%\x1B[u\r",
                    pos,
                    b.label,
                    "",
                    '>',
                    "",
                    100 * b.curr / b.total,
                    l = term_width - w - 8,
                    f = f,
                    e = e
                );
            }

            // Very important, or the output won't appear fluid.
            self.out.flush().unwrap();
        }
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
    label: String,
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
/// let bar = progress.bar(100, "Downloading");
/// progress.inc_and_draw(&bar, 1);
/// ```
///
/// As shown above, this type can only be constructed via [`Progress::bar`].
pub struct Bar(usize);
