use std::{
    fmt::Display,
    io::{prelude::BufRead, Write},
};

#[allow(clippy::cast_possible_truncation)]
pub fn floor(x: f64) -> i32 {
    x.floor() as i32
}

/// Digits after decimal point
/// # Panics
/// Panics if trucated float fails to be parsed
pub fn f_round(x: f32, digits: usize) -> f32 {
    format!("{x:.digits$}").parse().unwrap()
}

fn flush_stout() {
    std::io::stdout().flush().ok();
}

pub fn number_to_comma_sep_string<T: num_format::ToFormattedStr>(x: &T) -> String {
    // Create a stack-allocated buffer...
    let mut buf = num_format::Buffer::default();

    // Write "1,000,000" into the buffer...
    buf.write_formatted(x, &num_format::Locale::en);

    // Get a view into the buffer as a &str...
    buf.to_string()
}

pub trait Input {
    /// User input convenience function
    fn input<S: AsRef<str> + Display>(&self, message: S) -> String {
        println!("{message}");
        print!(">"); // Caret for user input
        flush_stout(); // To make sure

        let stdin = std::io::stdin();
        let mut buffer: String = String::new();
        match &stdin.lock().read_line(&mut buffer) {
            Ok(_) => buffer,
            Err(e) => panic!("{e}"),
        }
    }
}

/// User input convenience function
/// # Panics
/// Panics if file read fails on a particular line.
impl Input for String {
    fn input<S: AsRef<str> + Display>(&self, message: S) -> String {
        println!("{message}");
        print!(">"); // Caret for user input
        flush_stout(); // To make sure

        let stdin = std::io::stdin();
        let mut buffer: String = String::new();
        match &stdin.lock().read_line(&mut buffer) {
            Ok(_) => buffer,
            Err(e) => panic!("{}", e),
        }
    }
}
