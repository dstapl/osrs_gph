//use std::io::BufReader;

// Modules
pub mod api;
pub mod file_io;
pub mod helpers;
pub mod item_search;
pub mod prices;
pub mod types;

// TODO: Create folder for different front-end results
//      GUI, markdown file, etc.
pub mod results_writer;

// TODO: Migrate contents to lib.rs instead?
pub mod config;

use std::sync::Mutex;

use tracing::{debug, error, level_filters::LevelFilter, Level};
use tracing_subscriber::{
    layer::SubscriberExt,
    Layer, // prelude::*,
};

/// # Panics
/// Will panic on matching error branch
pub fn log_match_panic<R: std::any::Any + std::fmt::Debug, E: std::any::Any + std::fmt::Debug>(
    expr: Result<R, E>,
    desc: &str,
    err_msg: &str,
) -> R {
    let res: R = match expr {
        // TODO: Include result in log or not?
        //  Becomes very large for matching on files...
        Ok(res) => {
            debug!(desc = %desc);
            res
        }
        Err(e) => {
            log_panic(err_msg, e);
        }
    };

    res
}

/// # Panics
/// Intentionally panics after logging error message
pub fn log_panic<E: std::fmt::Debug>(desc: &str, reason: E) -> ! {
    error!(name = "PANIC", desc = %desc, reason = ?reason);
    panic!("{desc:?}");
}

/// # Panics
/// Will panic if fails to clear file contents (See [`FileIO::clear_contents`])
pub fn make_subscriber(filepath: String, log_level: Level) -> impl tracing::Subscriber {
    let mut log_file = file_io::FileIO::new(filepath, file_io::FileOptions::new(false, true, true));

    // Clear file now the subscriber is initialised
    log_file
        .clear_contents()
        .expect("Failed to clear log file contents");

    tracing_subscriber::registry().with(
        tracing_subscriber::fmt::layer()
            .with_ansi(false) // Disable colour codes in text
            .with_writer(Mutex::new(
                // TODO(Bug): When using custom FileIO some logs
                //  are truncated. May be due to using BufWriter?
                //  Since logs are not in order, the buffer gets flushed
                log_file.open_file().expect("Failed to open logging file"),
            ))
            .with_filter(LevelFilter::from_level(log_level)),
    )
}
