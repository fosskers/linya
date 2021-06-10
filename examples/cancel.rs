//! An example of cancelling a progress bar.
//!
//! See the `multi` example for details on overall usage of the library.

use linya::{Bar, Progress};
use rand::Rng;
use rayon::prelude::*;
use std::sync::Mutex;
use std::time::Duration;

fn main() {
    println!("Starting bars...");

    let progress = Mutex::new(Progress::new());

    (0..10).into_par_iter().for_each(|n| {
        let bar: Bar = progress
            .lock()
            .unwrap()
            .bar(50, format!("Downloading #{}", n));

        let mut rng = rand::thread_rng();
        let wait = rng.gen_range(25..250);

        for n in 0..=50 {
            // Simulate our "download" failing.
            let it_failed = rng.gen_range(0..=99) < 10;
            if it_failed {
                progress.lock().unwrap().cancel(bar);
                break;
            }

            progress.lock().unwrap().set_and_draw(&bar, n);
            std::thread::sleep(Duration::from_millis(wait));
        }
    });

    println!("Complete!");
}
