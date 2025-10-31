//! Front-end as a file
//! Implements traits from [`src/types.rs`]

pub mod markdown {
    use itertools::Itertools;
    use tracing::trace;

    use crate::types::{DetailedTable, OverviewRow, RecipeDetail, ResultsTable, DETAILED_NUM_HEADERS, DETAILED_ROW_HEADERS, OVERVIEW_NUM_HEADERS, OVERVIEW_ROW_HEADERS};
    use crate::helpers::ToCommaString;

    use std::io;



    pub mod optimal_overview {}

    // TODO: Is String best for large numbers of recipes?
    pub struct OptimalOverview {
        overview_rows: Vec<OverviewRow>,
        col_widths: [usize; OVERVIEW_NUM_HEADERS],
    }

    // TODO: Name conflict with src/recipes/* ?
    pub struct DetailedRecipeLookup {
        // TODO: Store here or pass in through function arguments?
        current_coins: i32, // User GP

        recipe_tables: Vec<DetailedTable>,
        current_table_idx: usize, // TODO: better way to do this?
        col_widths: [usize; DETAILED_NUM_HEADERS],

        // TODO: Vec<Option<_>>? or just keep as all entries are String::new()
        current_table_rows: Vec<[String; DETAILED_NUM_HEADERS]>, // Clear when switching tables
    }

    impl Default for OptimalOverview {
        fn default() -> Self {
            Self {
                overview_rows: Vec::new(), // TODO: Initialise with_capacity?
                col_widths: [0; OVERVIEW_NUM_HEADERS],
            }
        }
    }

    impl Default for DetailedRecipeLookup {
        fn default() -> Self {
            Self {
                current_coins: 0,

                recipe_tables: Vec::new(), // Ditto
                current_table_idx: 0,
                col_widths: [0; DETAILED_NUM_HEADERS],

                current_table_rows: Vec::new(),
            }
        } 
    }

    impl ResultsTable for OptimalOverview {
        type Row = OverviewRow;

        fn fmt_title(&self) -> Option<String> {
            None
        }

        fn table_separator(&self) -> String {
            // Optimal Overview only has one table
            String::new()
        }

        fn fmt_header(&self) -> String {
            format!(
                "| {:<width0$} | {:>width1$} | {:>width2$} | {:>width3$} | {:>width4$} |",
                OVERVIEW_ROW_HEADERS[0],
                OVERVIEW_ROW_HEADERS[1],
                OVERVIEW_ROW_HEADERS[2],
                OVERVIEW_ROW_HEADERS[3],
                OVERVIEW_ROW_HEADERS[4],
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

            // No title 

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

        // Same as `write_table`
        fn write_all_tables(&mut self, writer: &mut impl io::Write) -> io::Result<()> {
            // Only one table to write 
            self.write_table(writer)
        }
    }


    impl ResultsTable for DetailedRecipeLookup {
        // Printing tables at a time
        type Row = DetailedTable;

        fn fmt_title(&self) -> Option<String> {
            self.recipe_tables.get(self.current_table_idx)
                .map(|t| t.overview.name.clone())
        }

        /// TODO: better way to do this...? Not very scalable
        fn fmt_header(&self) -> String {
            format!(
                "| {:<width0$} | {:>width1$} | {:>width2$} | {:>width3$} | {:>width4$} | {:>width5$} | {:>width6$} |",
                DETAILED_ROW_HEADERS[0],
                DETAILED_ROW_HEADERS[1],
                DETAILED_ROW_HEADERS[2],
                DETAILED_ROW_HEADERS[3],
                DETAILED_ROW_HEADERS[4],
                DETAILED_ROW_HEADERS[5],
                DETAILED_ROW_HEADERS[6],
                width0 = self.col_widths[0],
                width1 = self.col_widths[1],
                width2 = self.col_widths[2],
                width3 = self.col_widths[3],
                width4 = self.col_widths[4],
                width5 = self.col_widths[5],
                width6 = self.col_widths[6],
            )
        }

        // Formats the internal table containing all table body sections
        // TODO: Change API of program... currently ignores the row argument...
        fn fmt_item(&self, _: &Self::Row) -> String {
            let current_internal_table = &self.current_table_rows;

            let mut body: Vec<String> = Vec::with_capacity(3 * Self::NUM_SECTION_HEADERS);
            for row in current_internal_table {
                let string_row = format!(
                    "| {:<width0$} | {:>width1$} | {:>width2$} | {:>width3$} | {:>width4$} | {:>width5$} | {:>width6$} |",
                    row[0],
                    row[1],
                    row[2],
                    row[3],
                    row[4],
                    row[5],
                    row[6],
                    width0 = self.col_widths[0],
                    width1 = self.col_widths[1],
                    width2 = self.col_widths[2],
                    width3 = self.col_widths[3],
                    width4 = self.col_widths[4],
                    width5 = self.col_widths[5],
                    width6 = self.col_widths[6],
                );

                body.push(string_row);
            }

            body.join("\n")
        }

        fn table_separator(&self) -> String {
            // Count actual cell content
            let inner_length: usize = self.col_widths.iter().sum();
            // Count boundary characters between cells
            // Each cell content x is padded by: | x | x | x |
            let boundary_length: usize = 3 * self.col_widths.len() + 1;
            let max_line_length: usize = inner_length + boundary_length;

            "#".repeat(max_line_length)
        }

        // Combines normal and price margin sections into a singular table
        fn write_table(&mut self, f: &mut impl io::Write) -> io::Result<()> {
            let current_internal_table = &self.recipe_tables[self.current_table_idx];

            // Print title
            writeln!(f, "{}\n", current_internal_table.overview.name)?;

            // Calculate cell padding
            // Creates the internal table body
            self.update_widths();

            // Print header
            writeln!(f, "{}", self.fmt_header())?;
            // and header separator
            let separator_cells = self.col_widths.iter().map(|w| "-".repeat(*w.max(&3)));
            writeln!(f, "| {} |", separator_cells.collect::<Vec<_>>().join(" | "))?;

            // TODO: Not working currently
            //  2025-10-28: Is this still the case?
            // Print internal table body
            let unused = &DetailedTable::default();
            writeln!(f, "{}", self.fmt_item(unused))?;

            // Print table separator
            writeln!(f, "\n{}\n", self.table_separator())?;

            // Clear internal variables
            self.current_table_rows.clear(); // Retains capacity for next table

            // Critical
            self.current_table_idx += 1; // Update for next table

            Ok(())
        }

        fn write_all_tables(&mut self, writer: &mut impl io::Write) -> io::Result<()> {
            let mut num_iterations: usize = 0;
            let num_tables: usize = self.recipe_tables.len();
            while let Some(table) = self.recipe_tables.get(self.current_table_idx) {
                // TODO: Redudancy...probably can remove
                if num_iterations >= num_tables {
                    break
                }

                let recipe_name = table.overview.name.clone();
                trace!(desc = "Writing recipe lookup table...", recipe = %recipe_name);
                self.write_table(writer)?;
                num_iterations += 1;
            };

            Ok(())
        }
    }

    impl OptimalOverview {
        pub fn new(overview_rows: Vec<OverviewRow>, col_widths: [usize; OVERVIEW_NUM_HEADERS]) -> Self {
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
            for (i, header) in OVERVIEW_ROW_HEADERS.iter().enumerate() {
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

    impl DetailedRecipeLookup {
        const NUM_SECTION_HEADERS: usize = 5;
        const BASE_HEADERS: [(&str, Option<&str>); Self::NUM_SECTION_HEADERS] = [
            ("Inputs", None),
            ("Total", None),
            ("Outputs", None),
            ("Total", Some("w/Tax")),
            ("Profit/Loss", Some("w/Tax")),
        ];

        pub fn new(current_gp: i32, tables: Vec<DetailedTable>, widths: [usize;DETAILED_NUM_HEADERS]) -> Self {
            Self {
                current_coins: current_gp,
                recipe_tables: tables,
                current_table_idx: 0,
                col_widths: widths,
                current_table_rows: Vec::new(),
            }

        }


        // TODO: Convert to [String; Self::NUM_SECTION_HEADERS instead of Vec<String>
        fn generate_section_headers(percent_margin: f32) -> [String; Self::NUM_SECTION_HEADERS] {
            assert_eq!(Self::BASE_HEADERS.len(), Self::NUM_SECTION_HEADERS);

            let section: Vec<_> = Self::BASE_HEADERS
                .iter()
                .map(|(label, extra)| {
                    let prefix = extra.unwrap_or("");
                    let margin_suffix = format!("{percent_margin}% margin");

                    if prefix.is_empty() { // e.g. `w/Tax`
                        format!("{label} (Base; {margin_suffix})")
                    } else {
                        format!("{label} ({prefix} Base; {margin_suffix})")
                    }
                }).collect();

            // SAFETY: Generating `section` will never fail, and iter length is asserted
            unsafe { section.try_into().unwrap_unchecked() }
        }


        fn push_input_rows(res: &mut Vec<[String; DETAILED_NUM_HEADERS]>, inputs: &Vec<RecipeDetail>, number_recipes: i32) {
            for (name, price, quantity) in inputs {
                const ERROR_MARGIN: f64 = 1.0;

                let quantity = quantity.to_owned();
                let quantity_is_int = (f64::from(quantity as i32) - f64::from(quantity)).abs() < ERROR_MARGIN;
                
                let quantity_string = if quantity_is_int {
                    (quantity as i32).to_comma_sep_string()  
                } else { format!("{quantity:.1}") }; // TODO: 1 sig. fig. instead of decimal place


                let total_quantity = number_recipes as f32 * quantity;
                let total_quantity_is_int = (f64::from(total_quantity as i32) - f64::from(total_quantity)).abs() < ERROR_MARGIN;

                let total_quantity_string = if total_quantity_is_int {
                    (total_quantity as i32).to_comma_sep_string() 
                } else { format!("{total_quantity:.1}") }; // Ditto


                let row = [
                    name.to_owned(),
                    quantity_string,
                    total_quantity_string,
                    price.to_comma_sep_string(),
                    ((f64::from(total_quantity) * f64::from(*price)) as i32).to_comma_sep_string(),
                    String::new(),
                    String::new(),
                ];

                res.push(row);
            }
        }

        fn push_section_rows(res: &mut Vec<[String; DETAILED_NUM_HEADERS]>, section_headers: &[String; Self::NUM_SECTION_HEADERS],
            current_internal_table: &DetailedTable) {
            const BLANK_LINE: [String;DETAILED_NUM_HEADERS] = [const{String::new()};DETAILED_NUM_HEADERS];

            // Input items
            let mut header = BLANK_LINE;
            header[0].clone_from(&section_headers[0]);
            res.push(header);

            let number_recipe = current_internal_table.overview.number;
            Self::push_input_rows(res, &current_internal_table.inputs, number_recipe);

            // Inputs Total
            header = BLANK_LINE;
            header[0].clone_from(&section_headers[1]);
            let single_input_price =  current_internal_table.inputs.iter()
                .map(|(_,price,quantity)| (f64::from(*price) * f64::from(*quantity)) as i32).sum::<i32>();
            header[4] = (single_input_price * number_recipe).to_comma_sep_string();
            res.push(header);

            res.push(BLANK_LINE);

            // Outputs items
            header = BLANK_LINE;
            header[0].clone_from(&section_headers[2]);
            res.push(header);

            Self::push_input_rows(res, &current_internal_table.outputs, number_recipe);

            // Outputs Total (Taxed)
            header = BLANK_LINE;
            header[0].clone_from(&section_headers[3]);
            let single_output_price =  current_internal_table.outputs.iter()
                .map(|(_,price,quantity)| (f64::from(*price) * f64::from(*quantity)) as i32).sum::<i32>();
            header[4] = (number_recipe * single_output_price).to_comma_sep_string();
            res.push(header);

            res.push(BLANK_LINE);

            // Profit/Loss
            header = [const{String::new()};DETAILED_NUM_HEADERS];
            header[0].clone_from(&section_headers[4]);
            header[4] = current_internal_table.overview.total_gp().to_comma_sep_string();
            header[5] = current_internal_table.overview.total_time().to_string();
            header[6] = current_internal_table.overview.gph().to_comma_sep_string();
            res.push(header);

        }


        // TODO: Move `push_section_rows` logic into here
        pub fn generate_section_rows(
            headers: &[String; Self::NUM_SECTION_HEADERS],
            table: &mut DetailedTable,
            // apply_margin: bool,
        ) -> Vec<[String; DETAILED_NUM_HEADERS]> {
            let mut rows = Vec::new();
            Self::push_section_rows(&mut rows, headers, table);

            rows
        }

        fn merge_rows<const N: usize>(base: [String; N], pm: [String; N]) -> [String; N] {
            base.into_iter()
                .zip(pm)
                .enumerate()
                .map(|(c, (b, p))| {
                    if (2 <= c) && !(b.is_empty() && p.is_empty()) {
                        return format!("{} ({})", b, p)
                    } else { b }
                })
            .collect::<Vec<_>>()
            .try_into()
            .unwrap()
        }

        fn adjust_prices(
            items: &[(String, i32, f32)],
            multiplier: f64,
        ) -> Vec<(String, i32, f32)> {
            items.iter()
            .map(|(name, price, qty)| {
                let adjusted = (f64::from(*price) * multiplier) as i32;
                (name.clone(), adjusted, *qty)
            })
            .collect()
        }

        pub fn create_internal_table_body_rows(&mut self) {
            let table = &mut self.recipe_tables[self.current_table_idx];
            let percent_margin = table.percent_margin; // 1.0 for 1.0% NOT 0.01
            let section_headers = Self::generate_section_headers( // Combined headings
                percent_margin
            );

            let base_rows = Self::generate_section_rows(&section_headers, table);
            



            // Price margin section
            // Update prices of inputs/outputs to reflect price margin

            // Increase buy prices and decrease sell prices
            table.inputs = Self::adjust_prices(&table.inputs, (1.0 + percent_margin/100.0).into());
            table.outputs = Self::adjust_prices(&table.outputs, (1.0 - percent_margin/100.0).into());




            // Generate PM section

            // Decrease number of recipe
            let input_cost_pm: i32 = DetailedTable::single_recipe_price(
                &table.inputs
            );
            table.overview.number = self.current_coins / input_cost_pm;

            // Decrease profit of recipe
            let output_cost_pm: i32 = DetailedTable::single_recipe_price(
                &table.outputs
            );
            table.overview.profit = output_cost_pm - input_cost_pm;

            let pm_rows = Self::generate_section_rows(&section_headers, table);


            // Combine into final result
            let res: Vec<[String; DETAILED_NUM_HEADERS]> = base_rows
                .into_iter()
                .zip(pm_rows)
                .map(|(b, p)| Self::merge_rows(b, p))
            .collect();

            self.current_table_rows = res;
        }


        fn _set_max_widths<I, T>(widths: &mut[usize; DETAILED_NUM_HEADERS],
            // new: [usize; DETAILED_NUM_HEADERS]) {
            new: I)
        where I: IntoIterator<Item = T>,
            T: Into<usize>
        {
            widths.iter_mut()
                .zip(new)
                .for_each(|(width, new_width)|
                    *width = (*width).max(new_width.into())
                );
        }

        /// Update `col_widths` with maximum cell widths across all rows
        /// # TODO
        /// Store results so not recalculating *EVERYTHING*
        pub fn update_widths(&mut self) {
            let table = &self.recipe_tables[self.current_table_idx];

            // Check table headers lengths
            Self::_set_max_widths(&mut self.col_widths,
                DETAILED_ROW_HEADERS.into_iter().map(str::len));


            // Check section headers for first column
            let section_headers = Self::generate_section_headers(
                table.percent_margin
            );

            if let Some(max_len) = section_headers.iter().map(String::len).max() {
                self.col_widths[0] = self.col_widths[0].max(max_len);
            } else { self.col_widths[0] = 0; }


            // Check all data rows lengths
            // Construct internal rows for current table
            self.create_internal_table_body_rows();


            for row in &self.current_table_rows {
                Self::_set_max_widths(&mut self.col_widths,
                    row.iter().map(|s| s.len()))
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
