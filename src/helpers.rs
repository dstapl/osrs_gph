use std::io::Write;

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

fn _flush_stout() {
    std::io::stdout().flush().ok();
}

pub trait ToCommaString {
    fn to_comma_sep_string(self) -> String;
}

impl<T: num_format::ToFormattedStr> ToCommaString for T {
    fn to_comma_sep_string(self) -> String {
        let mut buf = num_format::Buffer::default();

        // Format number as comma-separated
        buf.write_formatted(&self, &num_format::Locale::en);

        buf.to_string()
    }
}

