//! An example of using multiple `Progress` bars with natively spawned threads
//! and not Rayon.

use linya::Progress;
use rand::Rng;
use std::sync::{Arc, Mutex};
use std::time::Duration;

const BAR_MAX: usize = 1234;

fn main() -> std::thread::Result<()> {
    // Unlike with Rayon, we need to use an `Arc` here to allow the `move`s
    // below to work.
    let p0 = Arc::new(Mutex::new(Progress::new()));
    let p1 = p0.clone();

    let child0 = std::thread::spawn(move || {
        let bar = p0.lock().unwrap().bar(BAR_MAX, format!("Downloading #0"));

        // Determine how fast our thread progresses.
        let wait = rand::thread_rng().gen_range(1..=10);

        for n in 0..=BAR_MAX {
            // Only draws the line of the specified `Bar` without wasting
            // resources on the others.
            p0.lock().unwrap().set_and_draw(&bar, n);
            std::thread::sleep(Duration::from_millis(wait));
        }
    });

    // The same as the above.
    let child1 = std::thread::spawn(move || {
        let bar = p1.lock().unwrap().bar(BAR_MAX, format!("Downloading #1"));
        let wait = rand::thread_rng().gen_range(1..=10);

        for n in 0..=BAR_MAX {
            p1.lock().unwrap().set_and_draw(&bar, n);
            std::thread::sleep(Duration::from_millis(wait));
        }
    });

    child0.join()?;
    child1.join()
}
