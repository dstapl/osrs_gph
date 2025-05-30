//use std::io::BufReader;

// Modules
pub mod api;
pub mod file_io;
pub mod types;
pub mod item_search;
pub mod prices;
pub mod helpers;

// TODO: Migrate contents to lib.rs instead?
pub mod config;


use tracing::{debug, error, Level};
// Custom things
// #[macro_export]
// macro_rules! log_match_err {
//     () => {
//
//     };
// }
pub fn log_match_err<R: std::any::Any + std::fmt::Debug, E: std::any::Any + std::fmt::Debug>(expr: Result<R, E>, desc: &str, err_msg: &str) -> R{
    let res: R = match expr {
        Ok(res) => {debug!(desc = ?desc, result = ?res); res},
        Err(e) => {
            error!(desc = ?err_msg, reason = ?e);
            panic!("{}", err_msg);
        },
    };

    res
}

pub fn make_subscriber(filepath: String, log_level: Level) -> impl tracing::Subscriber {
    let log_file_options = file_io::FileOptions::new(false, true, true);
    // Cloning because "borrowed data leaves the function"
    let log_file = file_io::FileIO::new(filepath, log_file_options);

    let subscriber = tracing_subscriber::fmt()
        .with_writer(std::sync::Mutex::new(log_file))
        .finish();

    subscriber
}
