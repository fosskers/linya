use linya::{Bar, Progress};
use rand::Rng;
use std::fmt::Write;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const NUM_IMAGES: usize = 124;

fn main() {
    println!("Starting bars...");

    // `Progress` on its own can't be passed between threads, so we wrap it in
    // the usual sharing types.
    let progress = Arc::new(Mutex::new(Progress::new()));

    // We move `progress` into the `downl_thread` so we'll create another
    // reference using `Arc::clone`.
    let progress2 = progress.clone();

    let downl_thread = thread::spawn(move || {
        let bar: Bar = progress
            .lock()
            .unwrap()
            .bar(NUM_IMAGES, "Downloading Images");

        for i in 0..NUM_IMAGES {
            std::thread::sleep(Duration::from_millis(10));

            // Simulate our download failing
            if rand::thread_rng().gen_ratio(1, 20) {
                // Here we grab a handle to the part of stderr above the
                // progress bars and write to it.
                writeln!(
                    progress.lock().unwrap().stderr(),
                    "Image #{:03}: Downloading failed.",
                    i
                )
                .unwrap();
            }

            progress.lock().unwrap().inc_and_draw(&bar, 1);
        }
    });

    let process_thread = thread::spawn(move || {
        let bar: Bar = progress2
            .lock()
            .unwrap()
            .bar(NUM_IMAGES, "Post-processing Images");

        for i in 0..NUM_IMAGES {
            std::thread::sleep(Duration::from_millis(17));

            // Simulate our resizing failing
            // because we expected a square image.
            if rand::thread_rng().gen_ratio(1, 25) {
                writeln!(
                    progress2.lock().unwrap().stderr(),
                    "Image #{:03}: Not square.",
                    i
                )
                .unwrap();
            }

            progress2.lock().unwrap().inc_and_draw(&bar, 1);
        }
    });

    downl_thread.join().unwrap();
    process_thread.join().unwrap();

    println!("Complete!");
}
