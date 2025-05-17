//use std::io::BufReader;

// Modules
pub mod api;
pub mod file_io;
pub mod types;

// TODO: Migrate contents to lib.rs instead?
pub mod config;


// Custom things
// #[macro_export]
// macro_rules! log_match_err {
//     () => {
//
//     };
// }
use tracing::{debug, error};
fn log_match_err<R: std::any::Any + std::fmt::Debug, E: std::any::Any + std::fmt::Debug>(expr: Result<R, E>, desc: &str, err_msg: &str) -> R{
    let res: R = match expr {
        Ok(res) => {debug!(desc = ?desc, result = ?res); res},
        Err(e) => {
            error!(desc = ?err_msg, reason = ?e);
            panic!("{}", err_msg);
        },
    };

    res
}
