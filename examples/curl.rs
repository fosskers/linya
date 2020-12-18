use curl::easy::Easy;
use linya::{Bar, Progress};

fn main() -> Result<(), curl::Error> {
    println!("Starting tarball download...");

    let url = "";
    let mut progress = Progress::new();

    // In order to set the target total, you would need to know how big the data
    // was ahead of time.
    let bar: Bar = progress.bar(50, "Downloading...");

    // Establish our CURL settings.
    let mut handle = Easy::new();
    handle.url(url)?;
    handle.progress(true)?;

    // `progress_function` has aggressive lifetimes and requires the mutable
    // `progress` to be moved.
    handle.progress_function(move |_, downloaded, _, _| {
        progress.set_and_draw(&bar, downloaded as usize);
        true
    })?;

    // This would actually download the tarball if we had given a real URL, but
    // the bytes wouldn't be written anywhere because we didn't specify a
    // `write_function`.
    handle.perform()?;

    println!("Complete!");
    Ok(())
}
