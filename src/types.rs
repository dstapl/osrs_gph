use std::io;

use crate::helpers::{f_round, ToCommaString};

pub const SECOND_PER_TICK: f32 = 0.6;
pub const SEC_IN_HOUR: u16 = 60 * 60;

pub const OVERVIEW_NUM_HEADERS: usize = 5;
pub const OVERVIEW_ROW_HEADERS: [&str; OVERVIEW_NUM_HEADERS] = [
    "Method",
    "Loss/Gain",
    "(Total) Loss/Gain",
    "Time (Hours)",
    "GP/h",
];

// TODO: Add an extra col at the start for profit/loss to be separated?
pub const DETAILED_NUM_HEADERS: usize = 7;
pub const DETAILED_ROW_HEADERS: [&str; DETAILED_NUM_HEADERS] = [
    "Item",
    // TODO: Name
    "(Single Recipe Quantity)", // For an individual recipe
    "Quantity to offer", // Multiplied by number of recipes possible given current GP
    "At Offer Price (GP)", // Individual recipe
    "Total Price (GP)",
    "Total Time (h)",
    "Profit/Recipe Time (GP/h)"
];

pub trait ResultsTable {
    type Row;

    // Print title of current table
    fn fmt_title(&self) -> Option<String>;

    fn fmt_header(&self) -> String;

    /// Formats a row for printing
    fn fmt_item(&self, row: &Self::Row) -> String;

    /// Print the separators between tables
    fn table_separator(&self) -> String;

    /// Create output of current internal table
    /// TODO: `io::Write` or Formatter
    /// # Errors
    /// Will error if writing to `writer` fails. Refer to `io::Error`.
    fn write_table(&mut self, writer: &mut impl io::Write) -> io::Result<()>;

    /// # Errors
    /// Will error if `Self::write_table` fails. Refer to `io::Error`
    fn write_all_tables(&mut self, writer: &mut impl io::Write) -> io::Result<()>;
}

// TODO: Include cost & revenue in as well? (profit = revenue - cost)
// So far would only be used in the optimal overview functions in `prices.rs`
/// Internal row format for optimal overview.
/// To be formatted to an io output
#[derive(Debug, Default, Clone)]
pub struct OverviewRow {
    // &str?
    pub name: String,
    // Actually all integers* but i32 can be accurately represented by f64
    // *apart from time which is in decimal hours (Lower abs value)
    // loss/gain, (total)loss/gain, time (hours), gph
    // Repeated values can be calculated by multiplying number of recipes made
    pub pay_once_total: Option<i32>,
    pub profit: i32, // Can be negative
    pub time_sec: Option<f32>,
    pub number: i32, // TODO: Cap at i32 limit if using u32
}


/// TODO: Name
/// Internal table? format for recipe lookup.
/// To be formatted to an io output
/// # Note
/// Output prices should already be taxed *PER ITEM*
/// Since this is what is traded on the GE.
#[derive(Debug, Default, Clone)]
pub struct DetailedTable {
    pub overview: OverviewRow,
    // Store item name, price, quantity FOR A SINGLE RECIPE
    pub inputs: TableInputs,
    pub outputs: Vec<(String, i32, f32)>, // Ditto
    pub percent_margin: f32, // 2.5% == 2.5
}
#[derive(Debug, Default, Clone)]
pub struct TableInputs {
    pub pay_once: Option<Vec<(String, i32, f32)>>,
    pub inputs: Vec<(String, i32, f32)>,
}

impl OverviewRow {
    /// Construct a new row 
    pub fn new(name: String, pay_once_total: Option<i32>, profit: i32, time_sec: Option<f32>, number: i32) -> Self {
        OverviewRow {
            name,
            pay_once_total,
            profit,
            time_sec,
            number,
        }
    }

    /// Total time in hours
    pub fn total_time(&self) -> Option<f32> {
        #[allow(clippy::cast_precision_loss)]
        // Number of recipes isn't going to be larger than 10,000 at most
        // This is well under f32 limit of 2^23.
        let unrounded: f32 = self.time_sec? * (self.number as f32) / f32::from(SEC_IN_HOUR);

        Some(f_round(unrounded, 2))
    }

    pub fn ideal_loss_gain(&self) -> i32 {
        self.profit
    }

    pub fn loss_gain(&self) -> i32 {
        self.ideal_loss_gain() - self.pay_once_total.unwrap_or(0)
    }

    pub fn ideal_total_gp(&self) -> i32 {
        self.ideal_loss_gain() * self.number
    }

    pub fn total_gp(&self) -> i32 {
        // Compensate for removing once
        self.ideal_total_gp() - self.pay_once_total.unwrap_or(0)
    }
    pub fn gph(&self) -> i32 {
        #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
        if self.time_sec.is_none() {
            // Use number_per_hour
            // self.number * self.total_gp()
            self.total_gp() // Per hour already
        } else {
            (
                f32::from(SEC_IN_HOUR) * self.profit as f32
                / unsafe { self.time_sec.unwrap_unchecked() }
            ).floor() as i32
        }
    }

    pub fn format_time_string(&self) -> String {
        // TODO: Use estimate from (number/number_per_hour) * hours
        // in unwrap_or
        self.total_time()
            .map(|t| t.to_string())
            .unwrap_or("1.0".to_string()) // Since number_per_hour
    }
    pub fn to_string_cells(&self) -> [String; OVERVIEW_NUM_HEADERS] {
        [
            self.name.clone(),
            self.loss_gain().to_comma_sep_string(),
            self.total_gp().to_comma_sep_string(),
            self.format_time_string(),
            self.gph().to_comma_sep_string(),
        ]
    }
}


pub type RecipeDetail = (String, i32, f32); // Item name, price, quantity
impl DetailedTable {
    pub fn new(overview: OverviewRow, inputs: TableInputs, outputs: Vec<RecipeDetail>, percent_margin: f32) -> Self {
       Self {
           overview,
           inputs,
           outputs,
           percent_margin,
       } 
    }

    // TODO: name: function is same for inputs and outputs
    pub fn single_amount(inputs: &[RecipeDetail]) -> f32 {
        inputs.iter()
            .map(|r| r.2) // Quantity
            .sum()
    }

    // TODO: Ditto naming
    pub fn single_recipe_price(inputs: &[RecipeDetail]) -> i32 {
        #[allow(clippy::cast_possible_truncation)]
        inputs.iter()
            // Price * Quantity
            .map(|(_, p, q)| (f64::from(*p) * f64::from(*q)) as i32)
            .sum()
    }

    // TODO: Maybe remove some of these methods?

    pub fn total_amount_of_recipe(&self) -> i32 {
        // Re-calculate
        self.overview.number
    }

    pub fn total_price(&self, inputs: &[RecipeDetail]) -> i32 {
        let single_price: i32 = Self::single_recipe_price(inputs);

        self.overview.number * single_price
    }

    pub fn total_time(&self) -> Option<f32> {
        self.overview.total_time()
    }
}
