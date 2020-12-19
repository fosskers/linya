# linya

[![Workflow Status](https://github.com/fosskers/linya/workflows/Tests/badge.svg)](https://github.com/fosskers/linya/actions?query=workflow%3A%22Tests%22)
[![](https://img.shields.io/crates/v/linya.svg)](https://crates.io/crates/linya)

Simple concurrent progress bars.

![](https://github.com/fosskers/linya/blob/master/screenshots/multi.gif?raw=true)

## Features

- Intuitive API.
- First-class support for `rayon`, etc.
- Efficient, allocation-free redraws.
- Addition of new subbars on-the-fly.
- Single-threaded multi-bars.
- Light-weight, only a single dependency.

## Usage

`linya` is designed around the multi-bar case, and unlike other progress bar
libraries, has no separate type for individual bars. Instead, we use the
`Progress` type, a "bar coordinator".

### Multi Bars

`Progress` does not implement `Clone` and must be wrapped in the usual
[concurrent sharing types][arcmutex] before being passed between threads:

```rust
use std::sync::{Arc, Mutex};
use linya::{Bar, Progress};
use rayon::prelude::*;

let progress = Arc::new(Mutex::new(Progress::new()));

// `into_par_iter()` is from `rayon`, and lets us parallelize some
// operation over a collection "for free".
(0..10).into_par_iter().for_each_with(progress, |p, n| {
  let bar: Bar = p.lock().unwrap().bar(50, format!("Downloading {}", n));

  // ... Your logic ...

  // Increment the bar and draw it immediately.
  // This is likely called in some inner loop or other closure.
  p.lock().unwrap().inc_and_draw(&bar, 10);
});
```

Notice that new bars are added on-the-fly from within forked threads. We
call `Progress::bar` to obtain a new "bar handle", and then pass that
handle back to the parent `Progress` when incrementing/drawing.

See `Progress::inc_and_draw` and `Progress::set_and_draw` to advance and
render the bars.

### Single Bars

`Progress` can also be used in a single-threaded context for individual
bars. The usage is the same, except that no locking is required:

```rust
use linya::{Bar, Progress};

let mut progress = Progress::new();
let bar: Bar = progress.bar(50, "Downloading");

// Use in a loop, etc.
progress.set_and_draw(&bar, 10);
```

In this way, you could even have multi-bars in a single-threaded context.

## Caveats

Some of the points below may be fixed in future releases.

- Your terminal must support ANSI codes.
- No dedicated render thread, to keep usage simple.
- No bar templating, to avoid dependencies.
- No other bar styling ([yet]).
- No "rates", since rerenders are not time-based.
- No bar clearing after completion.
- No spinners, also due to no sense of time.
- No dynamic resizing of bars if window size changes.

If you need more customizable progress bars and are willing to accept
heavier dependencies, please consider [indicatif].

Note also that using more than one `Progress` at the same time leads to
unspecified behaviour.

## Trivia

*Linya* is the Quenya word for "pool", as in a [beautiful mountain pool][mirrormere].

[mirrormere]: https://www.tednasmith.com/tolkien/durins-crown-and-the-mirrormere/
[arcmutex]: https://doc.rust-lang.org/stable/book/ch16-03-shared-state.html?#atomic-reference-counting-with-arct
[yet]: https://internals.rust-lang.org/t/fmt-dynamic-fill-character/13609
[indicatif]: https://lib.rs/crates/indicatif
