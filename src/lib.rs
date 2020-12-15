//! Simple concurrent progress bars.

use std::io::{Stdout, Write};

// - Name: linya (Quenya for "pool"), barred
// - Focus on multibar support.
// - Works with ~into_iter()~ out of the box
// - No need to manually ~listen()~. The first ~inc()~ or ~set()~ of a subbar
// starts everything.
// - Bars auto-align
// - New bars are added on the fly
// - Only shows a bar if the process is active
// - Single bar completion is detected automatically
// - Replicate ILoveCandy
// - No (or few) dependencies
// - Spinners...?
// - No concept of singular bars? There is only one ~Progress~ type from which you
// request a bar entry, and then are returned a ~Handle~, etc?
// - *Bar doesn't draw in a separate thread, it blocks in the thread you're in and
// draws the "frame".* Naturally it locks as it does this. Hm, no, perhaps that's too
// inefficient. Bar updates could come very fast across all threads, but we don't want
// that to invoke a draw each time.

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
/// This type has no meaning methods of its own. Individual bars are advanced by
/// method calls on `Progress`:
///
/// ```
/// use linya::Progress;
///
/// let mut progress = Progress::new();
/// let bar = progress.bar(100);
/// progress.inc(&bar);
/// ```
pub struct Bar(usize);

//     curr: usize,
//     total: usize,
//     done: bool,
//     filler: char,
//     head: char,
//     empty: char,
// }

// impl SubBar {
//     /// Form a new single-threaded progress bar.
//     fn new(total: usize) -> SubBar {
//         SubBar {
//             curr: 0,
//             total,
//             done: false,
//             filler: '#',
//             head: '>',
//             empty: '-',
//         }
//     }

//     /// Increase the progress by 1 unit.
//     fn inc(&mut self) {
//         self.set(self.curr + 1)
//     }

//     /// Set the bar to a specific progress position.
//     fn set(&mut self, value: usize) {
//         if !self.done {
//             self.curr = value;
//             if self.curr >= self.total {
//                 self.done = true;
//             }

//             // self.draw();
//         }
//     }

//     // /// Draw the progress bar to the screen at its expected position.
//     // fn draw(&self) {
//     //     if !self.done {
//     //         println!("{}", self.to_string());
//     //     }
//     // }

//     // pub fn is_done(&self) -> bool {
//     //     self.done
//     // }

//     // /// ```
//     // /// use linya::Progress;
//     // ///
//     // /// let mut p = Progress::new(100);
//     // /// p.set(10);
//     // /// let expected = "[#####>---------------------------------------------]";
//     // /// assert!(!p.is_done());
//     // /// assert_eq!(expected.to_string(), p.to_string());
//     // /// p.set(100);
//     // /// let res = p.to_string();
//     // /// assert_eq!(52, res.chars().count());
//     // /// assert_eq!(expected.to_string(), p.to_string());
//     // /// ```
//     // pub fn to_string(&self) -> String {
//     //     if self.done {
//     //         format!("[{:#>f$}]", "", f = 50)
//     //     } else {
//     //         let f = 50 * self.curr / self.total;
//     //         let e = 50 - f;
//     //         format!("[{:#>f$}{}{:->e$}]", "", self.head, "", f = f, e = e)
//     //     }
//     // }
// }
