use linya::{Bar, Progress};
use rand::Rng;
use rayon::prelude::*;
use std::sync::{Arc, Mutex};
use std::time::Duration;

const BAR_MAX: usize = 1234;

fn main() {
    println!("Starting bars...");

    // `Progress` on its own can't be passed between threads, so we wrap it in
    // the usual sharing types.
    let progress = Arc::new(Mutex::new(Progress::new()));

    // `for_each_with` and similar Rayon functions let us pass some `Clone`able
    // value to each concurrent operation. In this case, it's our Arc-wrapped
    // progress bar coordinator.
    (0..10).into_par_iter().for_each_with(progress, |p, n| {
        // Create a new bar handle. This itself is not a progress bar type as
        // found in similar libraries! Notice below that the increment/draw
        // calls are done on the parent `Progress` type, not this `Bar`.
        let bar: Bar = p
            .lock()
            .unwrap()
            .bar(BAR_MAX, format!("Downloading #{}", n));

        // Determine how fast our thread progresses.
        let wait = rand::thread_rng().gen_range(1, 10);

        for n in 0..=BAR_MAX {
            // Only draws the line of the specified `Bar` without wasting
            // resources on the others.
            p.lock().unwrap().set_and_draw(&bar, n);

            std::thread::sleep(Duration::from_millis(wait));
        }
    });

    println!("Complete!");
}
