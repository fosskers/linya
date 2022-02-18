//! Simple concurrent progress bars.
//!
//! # Features
//!
//! - Intuitive API.
//! - First-class support for `rayon`, etc.
//! - Efficient, allocation-free redraws.
//! - Addition of new subbars on-the-fly.
//! - Single-threaded multi-bars.
//! - Light-weight, only a single dependency.
//!
//! # Usage
//!
//! `linya` is designed around the multi-bar case, and unlike other progress bar
//! libraries, has no separate type for individual bars. Instead, we use the
//! [`Progress`] type, a "bar coordinator".
//!
//! ## Multi Bars
//!
//! To mutably access a [`Progress`] across threads it must be wrapped in the
//! usual [concurrent sharing types][arcmutex]. With `rayon` specifically, only
//! `Mutex` is necessary:
//!
//! ```
//! use std::sync::Mutex;
//! use linya::{Bar, Progress};
//! use rayon::prelude::*;
//!
//! let progress = Mutex::new(Progress::new());
//!
//! // `into_par_iter()` is from `rayon`, and lets us parallelize some
//! // operation over a collection "for free".
//! (0..10).into_par_iter().for_each(|n| {
//!   let bar: Bar = progress.lock().unwrap().bar(50, format!("Downloading {}", n));
//!
//!   // ... Your logic ...
//!
//!   // Increment the bar and draw it immediately.
//!   // This is likely called in some inner loop or other closure.
//!   progress.lock().unwrap().inc_and_draw(&bar, 10);
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
//! # Caveats
//!
//! Some of the points below may be fixed in future releases.
//!
//! - Your terminal must support ANSI codes.
//! - No dedicated render thread, to keep usage simple.
//! - No bar templating, to avoid dependencies.
//! - No other bar styling ([yet]).
//! - No "rates", since rerenders are not time-based.
//! - No bar clearing after completion.
//! - No spinners, also due to no sense of time.
//! - No dynamic resizing of bars if window size changes.
//!
//! If you need more customizable progress bars and are willing to accept
//! heavier dependencies, please consider [indicatif].
//!
//! Note also that using more than one `Progress` at the same time leads to
//! unspecified behaviour.
//!
//! # Trivia
//!
//! *Linya* is the Quenya word for "pool", as in a [beautiful mountain pool][mirrormere].
//!
//! [mirrormere]: https://www.tednasmith.com/tolkien/durins-crown-and-the-mirrormere/
//! [arcmutex]: https://doc.rust-lang.org/stable/book/ch16-03-shared-state.html?#atomic-reference-counting-with-arct
//! [yet]: https://internals.rust-lang.org/t/fmt-dynamic-fill-character/13609
//! [indicatif]: https://lib.rs/crates/indicatif

#![warn(missing_docs)]
#![doc(html_root_url = "https://docs.rs/linya/0.2.1")]

use std::io::{LineWriter, Stderr, Write};
use terminal_size::{terminal_size, Height, Width};

/// A progress bar "coordinator" to share between threads.
#[derive(Debug)]
pub struct Progress {
    /// The drawable bars themselves.
    bars: Vec<SubBar>,
    /// A shared handle to `Stderr`.
    ///
    /// Line-buffered so that the cursor doesn't jump around unpleasantly.
    out: LineWriter<Stderr>,
    /// Terminal width and height.
    size: Option<(usize, usize)>,
}

impl Default for Progress {
    fn default() -> Progress {
        Progress::new()
    }
}

// You will notice in a number of the methods below that `Result` values from
// calling `write!` are being ignored via a `let _ = ...` pattern, as opposed to
// unwrapping. This avoids a rare panic that can occur under very specific shell
// piping scenarios.
impl Progress {
    /// Initialize a new progress bar coordinator.
    pub fn new() -> Progress {
        let out = LineWriter::new(std::io::stderr());
        let bars = vec![];
        let size = terminal_size().map(|(Width(w), Height(h))| (w as usize, h as usize));
        Progress { bars, out, size }
    }

    /// Like [`Progress::new`] but accepts a size hint to avoid reallocation as bar count grows.
    pub fn with_capacity(capacity: usize) -> Progress {
        let out = LineWriter::new(std::io::stderr());
        let bars = Vec::with_capacity(capacity);
        let size = terminal_size().map(|(Width(w), Height(h))| (w as usize, h as usize));
        Progress { bars, out, size }
    }

    /// Create a new progress bar with default styling and receive an owned
    /// handle to it.
    ///
    /// # Panics
    ///
    /// Passing `0` to this function will cause a panic the first time a draw is
    /// attempted.
    pub fn bar<S: Into<String>>(&mut self, total: usize, label: S) -> Bar {
        let twidth = self.size.map(|(w, _)| w).unwrap_or(100);
        let w = (twidth / 2) - 7;
        let label: String = label.into();

        // An initial "empty" rendering of the new bar.
        let _ = writeln!(
            &mut self.out,
            "{:<l$}      [{:->f$}]   0%",
            label,
            "",
            l = twidth - w - 8 - 5,
            f = w
        );

        let bar = SubBar {
            curr: 0,
            prev: 0,
            total,
            label,
            cancelled: false,
        };
        self.bars.push(bar);
        Bar(self.bars.len() - 1)
    }

    /// Set a particular [`Bar`]'s progress value, but don't draw it.
    pub fn set(&mut self, bar: &Bar, value: usize) {
        self.bars[bar.0].curr = value;
    }

    /// Force the drawing of a particular [`Bar`].
    ///
    /// **Note 1:** Drawing will only occur if there is something meaningful to
    /// show. Namely, if the progress has advanced at least 1% since the last
    /// draw.
    ///
    /// **Note 2:** If your program is not being run in a terminal, an initial
    /// empty bar will be printed but never refreshed.
    pub fn draw(&mut self, bar: &Bar) {
        // If there is no legal width value present, that means we aren't
        // running in a terminal, and no rerendering can be done.
        if let Some((term_width, term_height)) = self.size {
            let pos = self.bars.len() - bar.0;

            // For now, if the progress for a particular bar is slow and drifts
            // past the top of the terminal, redrawing is paused.
            if pos < term_height {
                let mut b = &mut self.bars[bar.0];
                let w = (term_width / 2) - 7;
                let (data, unit) = denomination(b.curr);
                let diff = 100 * ((b.curr - b.prev) / b.total);

                if b.cancelled {
                    let _ = write!(
                        &mut self.out,
                        "\x1B[s\x1B[{}A\r{:<l$} {:3}{} [{:_>f$}] ???%\x1B[u\r",
                        pos,
                        b.label,
                        data,
                        unit,
                        "",
                        l = term_width - w - 8 - 5,
                        f = w,
                    );

                    // Very important, or the output won't appear fluid.
                    let _ = self.out.flush();
                } else if b.curr >= b.total {
                    let _ = write!(
                        &mut self.out,
                        "\x1B[s\x1B[{}A\r{:<l$} {:3}{} [{:#>f$}] 100%\x1B[u\r",
                        pos,
                        b.label,
                        data,
                        unit,
                        "",
                        l = term_width - w - 8 - 5,
                        f = w,
                    );
                    let _ = self.out.flush();
                } else if diff >= 1 {
                    b.prev = b.curr;
                    let f = (w * (b.curr / b.total)).min(w - 1);
                    let e = (w - 1) - f;

                    let _ = write!(
                        &mut self.out,
                        "\x1B[s\x1B[{}A\r{:<l$} {:3}{} [{:#>f$}>{:->e$}] {:3}%\x1B[u\r",
                        pos,
                        b.label,
                        data,
                        unit,
                        "",
                        "",
                        100 * (b.curr / b.total),
                        l = term_width - w - 8 - 5,
                        f = f,
                        e = e
                    );
                    let _ = self.out.flush();
                }
            }
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

    /// Cancel the given bar, say in the case of download failure, etc.
    ///
    /// This fills the bar with the "cancel" character and consumes `Bar`
    /// ownership so that the bar cannot be manipulated again.
    pub fn cancel(&mut self, bar: Bar) {
        {
            let mut b = &mut self.bars[bar.0];
            b.cancelled = true;
        }
        self.set_and_draw(&bar, self.bars[bar.0].total);
    }
}

/// An internal structure that stores individual bar state.
#[derive(Debug)]
struct SubBar {
    /// Progress as of the previous draw.
    prev: usize,
    /// Current progress.
    curr: usize,
    /// The progress target.
    total: usize,
    /// A user-supplied label for the left side of the bar line.
    label: String,
    /// Did the user force this bar to stop?
    cancelled: bool,
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
#[derive(Debug)]
pub struct Bar(usize);

/// Reduce some raw byte count into a more human-readable form.
fn denomination(curr: usize) -> (usize, char) {
    match curr {
        _ if curr >= 1_000_000_000 => (curr / 1_000_000_000, 'G'),
        _ if curr >= 1_000_000 => (curr / 1_000_000, 'M'),
        _ if curr >= 1000 => (curr / 1000, 'K'),
        _ => (curr, ' '),
    }
}
