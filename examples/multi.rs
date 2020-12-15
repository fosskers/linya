use linya::{Bar, Progress};
use rand::Rng;
use rayon::prelude::*;
use std::sync::{Arc, Mutex};
use std::time::Duration;

fn main() {
    println!("Starting bars...");

    let progress = Arc::new(Mutex::new(Progress::new()));

    (0..10).into_par_iter().for_each_with(progress, |p, _| {
        let bar: Bar = { p.lock().unwrap().bar(50) };
        let wait = rand::thread_rng().gen_range(25, 250);

        for n in 0..=50 {
            {
                p.lock().unwrap().set_and_draw(&bar, n);
            }

            std::thread::sleep(Duration::from_millis(wait));
        }
    });

    println!("Complete!");
}
