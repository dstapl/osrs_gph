//! Front-end as a file
//! Implements traits from [`src/types.rs`]



pub mod markdown {
    use std::collections::HashMap;
    use crate::{helpers::number_to_comma_sep_string, item_search::{item_search::Item, recipes::{Recipe, RecipeTime}}, prices::prices::PriceHandle, types::{OverviewRow, OverviewTable, ResultsTable, NUM_HEADERS, ROW_HEADERS, SECOND_PER_TICK}};

    pub mod optimal_overview {
        use std::fmt;
    }

    // TODO: Is String best for large numbers of recipes?
    pub struct OptimalOverview { recipes: Vec<Recipe>, col_widths: [usize;NUM_HEADERS], table: String }
    // struct RecipeLookup { data: String }

    impl OptimalOverview {
        /// Update col_widths with maximum cell widths across all rows
        pub fn update_widths(&mut self) -> ! {
            todo!()
        }
    }

    impl Default for OptimalOverview {
        fn default() -> Self {
            Self {
                recipes: Vec::new(), // Empty
                col_widths: [0;NUM_HEADERS],
                table: String::with_capacity(100) // TODO: What in bytes?
            }
        }
    }

    impl ResultsTable for OptimalOverview {
        type Row = OverviewRow;
        type Table = Vec<Option<OverviewRow>>;

        fn table_separator(&self) -> String {
            // Optimal Overview only has one table
            String::new()
        }

        fn fmt_header(&self) -> String {
            format!(
                "| {:<width0$} | {:>width1$} | {:>width2$} | {:>width3$} | {:>width4$} |",
                ROW_HEADERS[0],
                ROW_HEADERS[1],
                ROW_HEADERS[2],
                ROW_HEADERS[3],
                ROW_HEADERS[4],
                width0 = self.col_widths[0],
                width1 = self.col_widths[1],
                width2 = self.col_widths[2],
                width3 = self.col_widths[3],
                width4 = self.col_widths[4],
            )
        }

        fn fmt_item(&self, row: Self::Row) -> String {
            format!(
                "| {:<width0$} | {:>width1$} | {:>width2$} | {:>width3$} | {:>width4$} |",
                row.name,
                number_to_comma_sep_string(&row.profit),
                number_to_comma_sep_string(&row.total_gp()),
                row.total_time(),
                number_to_comma_sep_string(&row.gph()),
                width0 = self.col_widths[0],
                width1 = self.col_widths[1],
                width2 = self.col_widths[2],
                width3 = self.col_widths[3],
                width4 = self.col_widths[4],
            )
        }

        fn create_table(&self) -> Self::Table {
           todo!(); 
        }
    }

    #[cfg(test)]
    mod test {
        use super::*;

        #[test]
        /// Want to check that the range of numbers stored in a row
        /// can be accurately displayed by the formatter.
        fn print_single_row_format() {
            let row = OverviewRow {
                name: "Humidify Clay".to_string(),
                profit: 375,
                time_sec: 3.6,
                number: 1_571,
            };
            let formatter = OptimalOverview::default();

            let expected = "| Humidify Clay | 375 | 589,125 | 1.57 | 375,000 |";
            assert_eq!(
                formatter.fmt_item(row),
                expected,
                "Check if the row/table format has changed."
            );
        }

        // #[test]
        // /// Check creation of a table
        // fn print_table() {
        //     todo!();
        //     // let table = Table{separator_value: "=".repeat(10), file_type: FileType::OptimalOverview};
        //     // println!("{table:?}");
        // }
    }
}





