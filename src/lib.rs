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
#![doc(html_root_url = "https://docs.rs/linya/0.3.0")]

use std::fmt;
use std::io::{BufWriter, Stderr, Write};
use terminal_size::{terminal_size, Height, Width};

/// A progress bar "coordinator" to share between threads.
#[derive(Debug)]
pub struct Progress {
    /// The drawable bars themselves.
    bars: Vec<SubBar>,
    /// A shared handle to `Stderr`.
    ///
    /// Buffered so that the cursor doesn't jump around unpleasantly.
    out: BufWriter<Stderr>,
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
        let out = BufWriter::new(std::io::stderr());
        let bars = vec![];
        let size = terminal_size().map(|(Width(w), Height(h))| (w as usize, h as usize));
        Progress { bars, out, size }
    }

    /// Like [`Progress::new`] but accepts a size hint to avoid reallocation as bar count grows.
    pub fn with_capacity(capacity: usize) -> Progress {
        let out = BufWriter::new(std::io::stderr());
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
            self.out,
            "{:<l$}      [{:->f$}]   0%",
            label,
            "",
            l = twidth - w - 8 - 5,
            f = w
        );
        let _ = self.out.flush();

        let bar = SubBar {
            curr: 0,
            prev_percent: 0,
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
        self.draw_impl(bar, false);

        // Very important, or the output won't appear fluid.
        let _ = self.out.flush();
    }

    /// Actually draw a particular [`Bar`].
    ///
    /// When `force` is true draw the bar at the current cursor position and
    /// advance the cursor one line.
    ///
    /// This function does not flush the output stream.
    fn draw_impl(&mut self, bar: &Bar, force: bool) {
        // If there is no legal width value present, that means we aren't
        // running in a terminal, and no rerendering can be done.
        if let Some((term_width, term_height)) = self.size {
            let pos = self.bars.len() - bar.0;
            let mut b = &mut self.bars[bar.0];
            let cur_percent = (100 * b.curr as u64) / (b.total as u64);
            // For a newly cancelled bar `diff` is equal to 100.
            let diff = cur_percent - b.prev_percent as u64;

            // For now, if the progress for a particular bar is slow and drifts
            // past the top of the terminal, redrawing is paused.
            if (pos < term_height && diff >= 1) || force {
                let w = (term_width / 2) - 7;
                let (data, unit) = denomination(b.curr);
                b.prev_percent = cur_percent as usize;

                if !force {
                    // Save cursor position and then move up `pos` lines.
                    let _ = write!(self.out, "\x1B[s\x1B[{}A\r", pos);
                }

                let _ = write!(
                    self.out,
                    "{:<l$} {:3}{} [",
                    b.label,
                    data,
                    unit,
                    l = term_width - w - 8 - 5,
                );
                if b.cancelled {
                    let _ = write!(self.out, "{:_>f$}] ??? ", "", f = w);
                } else if b.curr >= b.total {
                    let _ = write!(self.out, "{:#>f$}] 100%", "", f = w);
                } else {
                    let f = (((w as u64) * (b.curr as u64) / (b.total as u64)) as usize).min(w - 1);
                    let e = (w - 1) - f;

                    let _ = write!(
                        self.out,
                        "{:#>f$}>{:->e$}] {:3}%",
                        "",
                        "",
                        (100 * (b.curr as u64)) / (b.total as u64),
                        f = f,
                        e = e
                    );
                }

                if !force {
                    // Return to previously saved cursor position.
                    let _ = write!(self.out, "\x1B[u\r");
                } else {
                    let _ = writeln!(self.out);
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
            // Force redraw by setting `prev_percent` to 0.
            b.prev_percent = 0;
        }
        self.set_and_draw(&bar, self.bars[bar.0].total);
    }

    /// Return a handle to write above all progress bars.
    ///
    /// When the handle is dropped all progress bars are redrawn.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::fmt::Write;
    ///
    /// # use linya::Progress;
    /// # let mut progress = Progress::new();
    /// writeln!(progress.stderr(), "Some log message");
    /// ```
    pub fn stderr(&mut self) -> impl fmt::Write + '_ {
        // Move to first line of the progress bars, erase the complete line and print the message.
        let _ = write!(self.out, "\x1B[{}A\x1B[2K\r", self.bars.len()).map_err(|_e| fmt::Error);
        WriteHandle { prog: self }
    }
}

/// A write handle that exclusively holds a [`Progress`] instance so
/// that no draws can interfere with writing.
#[derive(Debug)]
struct WriteHandle<'a> {
    prog: &'a mut Progress,
}

impl<'a> fmt::Write for WriteHandle<'a> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.prog
            .out
            .write_all(s.as_bytes())
            .map_err(|_e| fmt::Error)
    }
}

impl<'a> Drop for WriteHandle<'a> {
    fn drop(&mut self) {
        // Determine first bar that fits on screen.
        let start = self
            .prog
            .size
            .map(|(_w, h)| {
                if self.prog.bars.len() >= h {
                    self.prog.bars.len() - (h - 1)
                } else {
                    0
                }
            })
            .unwrap_or_default();

        // Redraw all progress bars.
        for bar in start..self.prog.bars.len() {
            self.prog.draw_impl(&Bar(bar), true);
        }

        // Flush all of them at once to reduce stutter.
        let _ = self.prog.out.flush();
    }
}

/// An internal structure that stores individual bar state.
#[derive(Debug)]
struct SubBar {
    /// Progress as of the previous draw in percent.
    prev_percent: usize,
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
