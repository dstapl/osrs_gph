//! Front-end as a file
//! Implements traits from [`src/types.rs`]

pub mod markdown {
    use crate::types::{OverviewRow, ResultsTable, NUM_HEADERS, ROW_HEADERS};
    use std::io;

    pub mod optimal_overview {}

    // TODO: Is String best for large numbers of recipes?
    pub struct OptimalOverview {
        overview_rows: Vec<OverviewRow>,
        col_widths: [usize; NUM_HEADERS],
    }
    // TODO: struct RecipeLookup { data: String }

    impl Default for OptimalOverview {
        fn default() -> Self {
            Self {
                overview_rows: Vec::new(), // TODO: Initialise with_capacity?
                col_widths: [0; NUM_HEADERS],
            }
        }
    }

    impl ResultsTable for OptimalOverview {
        type Row = OverviewRow;

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

        fn fmt_item(&self, row: &Self::Row) -> String {
            let string_cells = row.to_string_cells();

            format!(
                "| {:<width0$} | {:>width1$} | {:>width2$} | {:>width3$} | {:>width4$} |",
                string_cells[0],
                string_cells[1],
                string_cells[2],
                string_cells[3],
                string_cells[4],
                width0 = self.col_widths[0],
                width1 = self.col_widths[1],
                width2 = self.col_widths[2],
                width3 = self.col_widths[3],
                width4 = self.col_widths[4],
            )
        }

        /// TODO: Name
        /// Format table and write to ouptut
        fn write_table(&mut self, f: &mut impl io::Write) -> io::Result<()> {
            // Update column widths
            self.update_widths();

            // Write header row
            writeln!(f, "{}", self.fmt_header())?;

            // Write separator row
            let separator_cells = self.col_widths.iter().map(|w| "-".repeat(*w.max(&3)));
            writeln!(f, "| {} |", separator_cells.collect::<Vec<_>>().join(" | "))?;

            // Write data rows
            for row in &self.overview_rows {
                writeln!(f, "{}", self.fmt_item(row))?;
            }

            Ok(())
        }
    }

    impl OptimalOverview {
        pub fn new(overview_rows: Vec<OverviewRow>, col_widths: [usize; NUM_HEADERS]) -> Self {
            OptimalOverview {
                overview_rows,
                col_widths,
            }
        }

        /// Update `col_widths` with maximum cell widths across all rows
        /// # TODO
        /// Store results so not recalculating *EVERYTHING*
        pub fn update_widths(&mut self) {
            // Check headers lengths
            for (i, header) in ROW_HEADERS.iter().enumerate() {
                self.col_widths[i] = self.col_widths[i].max(header.len());
            }

            // Check all data rows lengths
            for row in &self.overview_rows {
                let string_cells = row.to_string_cells();

                for (width, cell) in self.col_widths.iter_mut().zip(string_cells.iter()) {
                    *width = (*width).max(cell.len());
                }
            }
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
                formatter.fmt_item(&row),
                expected,
                "Check if the row/table format has changed."
            );
        }

        // #[test]
        // /// Check creation of a table
        // fn write_table() {
        //     todo!();
        //     // let table = Table{separator_value: "=".repeat(10), file_type: FileType::OptimalOverview};
        //     // println!("{table:?}");
        // }
    }
}
