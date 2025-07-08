use std::io;

use crate::helpers::{f_round, ToCommaString};

pub const SECOND_PER_TICK: f32 = 0.6;
pub const SEC_IN_HOUR: u16 = 60 * 60;
pub const NUM_HEADERS: usize = 5;
pub const ROW_HEADERS: [&str; NUM_HEADERS] = [
    "Method",
    "Loss/Gain",
    "(Total) Loss/Gain",
    "Time (Hours)",
    "GP/H",
];

pub trait ResultsTable {
    type Row;

    /// Print the separators between tables
    fn table_separator(&self) -> String;
    /// Formats a row for printing
    fn fmt_item(&self, row: &Self::Row) -> String;
    fn fmt_header(&self) -> String;

    /// Create output of current internal table
    /// TODO: `io::Write` or Formatter
    /// # Errors
    /// Will error if writing to `writer` fails. Refer to `io::Error`.
    fn write_table(&mut self, writer: &mut impl io::Write) -> io::Result<()>;
}

/// Internal row format to be processed on output
#[derive(Debug, Default, Clone)]
pub struct OverviewRow {
    // &str?
    pub name: String,
    // Actually all integers* but i32 can be accurately represented by f64
    // *apart from time which is in decimal hours (Lower abs value)
    // loss/gain, (total)loss/gain, time (hours), gph
    // Repeated values can be calculated by multiplying number of recipes made
    pub profit: i32, // Can be negative
    pub time_sec: f32,
    pub number: i32, // TODO: Cap at i32 limit if using u32
}
/// Internal table
pub type OverviewTable = Vec<OverviewRow>; // Blank Rows are None

impl OverviewRow {
    /// Total time in hours
    pub fn total_time(&self) -> f32 {
        #[allow(clippy::cast_precision_loss)]
        // Number of recipes isn't going to be larger than 10,000 at most
        // This is well under f32 limit of 2^23.
        let unrounded: f32 = self.time_sec * (self.number as f32) / f32::from(SEC_IN_HOUR);

        f_round(unrounded, 2)
    }
    pub fn total_gp(&self) -> i32 {
        self.profit * self.number
    }
    pub fn gph(&self) -> i32 {
        #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
        return (f32::from(SEC_IN_HOUR) * self.profit as f32 / self.time_sec).floor() as i32;
    }

    pub fn to_string_cells(&self) -> [String; NUM_HEADERS] {
        [
            self.name.clone(),
            self.profit.to_comma_sep_string(),
            self.total_gp().to_comma_sep_string(),
            self.total_time().to_string(),
            self.gph().to_comma_sep_string(),
        ]
    }
}
