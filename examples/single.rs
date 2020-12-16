use linya::{Bar, Progress};
use std::time::Duration;

fn main() {
    println!("Starting bar...");

    // `Progress` is not a bar, but a "bar coordinator".
    let mut progress = Progress::new();

    // An owned handler to an internal bar.
    let bar: Bar = progress.bar(50);

    for n in 0..=50 {
        // Incrementing/drawing calls aren't done on `Bar`, but on the parent
        // `Progress`.
        progress.set_and_draw(&bar, n);

        std::thread::sleep(Duration::from_millis(60));
    }

    println!("Complete!");
}
