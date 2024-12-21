use std::fmt;
use thousands::Separable;

const SEC_IN_HOUR: u16 = 60 * 60;
const NUM_HEADERS: usize = 5;
const ROW_HEADERS: [&str; NUM_HEADERS] = [
    "Method",
    "Loss/Gain",
    "(Total) Loss/Gain",
    "Time (Hours)",
    "GP/H",
];

trait FileTable {
    /// Print the separators between tables
    fn table_separator(&self) -> String;
    /// Formats a row for printing
    fn fmt_item<T>(&self, row: T) -> String;
}

// struct Table {
//     separator_value: String,
// }
// struct OptimalOverview(Table);
// struct RecipeLookup(Table);

// impl FileTable for OptimalOverview{
//     fn fmt_item<T>(&self, row: T) -> String {

//     }
//     fn table_separator(&self) -> String {

//     }
// }
// impl FileTable for RecipeLookup{
//     fn fmt_item(&self, row: Row) -> String {

//     }
//     fn table_separator(&self) -> String {

//     }
// }
// impl FileTable for Table {
//     fn table_separator(&self) -> String {
//         match self.file_type {
//             FileType::OptimalOverview => self.separator_value.clone() + ",",
//             FileType::RecipeLookup => format!("|{}|", self.separator_value.replace(',', "|")),
//         }
//     }

//     fn fmt_item(&self, row: Row) -> String {
//         match self.file_type {
//             FileType::OptimalOverview => format!("{},", row),
//             FileType::RecipeLookup => format!("| {} |", row),
//         }
//     }
// }

#[derive(Debug, Default)]
pub struct Row {
    // &str?
    name: String,
    // Actually all integers* but i32 can be accurately represented by f64
    // *apart from time which is in decimal hours (Lower abs value)
    // loss/gain, (total)loss/gain, time (hours), gph
    // Repeated values can be calculated by multiplying number of recipes made
    recipe_gp: i32,
    recipe_time: Option<f32>,
    number: i32,
}

impl Row {
    /// Total time in hours
    fn total_time(&self) -> Option<f32> {
        // Number of recipes isn't going to be larger than 10,000 at most
        // This is well under f32 limit of 2^23.
        Some(self.recipe_time? * (self.number as f32) / SEC_IN_HOUR as f32)
    }
    fn total_gp(&self) -> i32 {
        self.recipe_gp * self.number
    }
    fn gph(&self) -> Option<i32> {
        Some((SEC_IN_HOUR as f32 * self.recipe_gp as f32 / self.recipe_time?) as i32)
    }
}

impl std::fmt::Display for Row {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "|{}|{}|{}|{}|{}|",
            self.name,
            self.recipe_gp,
            self.total_gp(),
            self.total_time()
                .map_or_else(|| "".to_string(), |t| t.to_string()),
            self.gph().map_or_else(|| "".to_string(), |t| t.to_string()),
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    /// Want to check that the range of numbers stored in a row
    /// can be accurately displayed by the formatter.
    fn print_single_row_format() {
        let row = Row {
            name: "Humidify Clay".to_string(),
            recipe_gp: 375,
            recipe_time: Some(3.6),
            number: 1_571,
        };

        let expected = "| Humidify Clay |       375 |         589,125 |     1.57 | 375,000 |";
        assert_eq!(
            format!("{row}"),
            expected,
            "Check if the row/table format has changed."
        );
    }

    #[test]
    /// Check creation of a table
    fn print_table() {
        todo!();
        // let table = Table{separator_value: "=".repeat(10), file_type: FileType::OptimalOverview};
        // println!("{table:?}");
    }
}
