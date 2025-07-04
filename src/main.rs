//! TODO: 2025-06-16 Something weird is happening with the log...
//! Unless the file is manually cleared, the contents will still remain

use std::{
    collections::HashMap,
    io::{self, Write},
};

use osrs_gph::{
    api::Api,
    config,
    file_io::{FileIO, FileOptions},
    helpers::Input,
    item_search::recipes::RecipeBook,
    log_match_panic, log_panic,
    prices::prices::PriceHandle,
    types::ROW_HEADERS,
};
use tracing::{info, instrument, span, trace, warn, Level};

fn main() {
    let conf: config::Config = config::load_config("config.yaml");

    // Level:: ERROR, INFO, TRACE
    // Span levels are akin to the event levels:
    //     too high and will revert to default guard instead of the span
    const LOG_LEVEL: Level = Level::TRACE;
    let subscriber = osrs_gph::make_subscriber(conf.filepaths.main_log_file.clone(), LOG_LEVEL);

    let _crateguard = tracing::subscriber::set_default(subscriber);
    let span = span!(LOG_LEVEL, "main");
    let _guard = span.enter();

    trace!(desc = "Loaded config and created subscriber to log file.");

    // Initialise with price data file path
    let mut file = FileIO::new(
        conf.filepaths.price_data.clone(),
        FileOptions::new(true, true, true),
    );

    trace!(desc = "Taking user input...");
    let inp = String::new().input("1. API Refresh Data\n2. Load previous Data\n");
    trace!(raw_input = %inp);

    let choice = inp.trim_end();
    trace!(choice = %choice);

    // Referesh API prices
    match choice {
        "1" => {
            info!(desc = "Retrieving prices from API.");
            request_new_prices_from_api(&conf.api, &mut file);
        }
        "2" => {
            info!(desc = "Loading previous data instead.");
        }
        _ => log_panic("Bad choice", choice),
    }

    // Create item search
    let mut item_search = osrs_gph::item_search::item_search::ItemSearch::new(
        HashMap::new(), // Empty items list
        conf.filepaths.clone(),
        conf.api,
    );

    // Populate with items (from_file)
    let item_prices = item_search.get_item_prices(true);
    item_search.update_item_prices(item_prices);

    // Get ignored items from the config
    let ignore_items: Vec<String> = conf.profit.ignore_items.clone();

    // Remove items contained in ignore_items
    item_search.ignore_items(&ignore_items);

    // Load in recipes
    let mut recipe_list = RecipeBook::new(HashMap::new());
    recipe_list.load_default_recipes(conf.filepaths.lookup_data.recipes);

    trace!(desc = "Creating price handle...");
    let price_handle = PriceHandle::new(
        item_search,
        recipe_list,
        conf.profit.coins,
        conf.profit.percent_margin,
    );

    trace!(desc = "Computing weights for pareto sort...");
    let weights = osrs_gph::prices::pareto_sort::custom_types::compute_weights(
        conf.profit.coins,
        &conf.profit.weights,
    );

    trace!(desc = "Creating all recipe overview");
    let optimal_overview = price_handle.all_recipe_overview(&weights, &conf.display);
    assert!(!optimal_overview.is_empty());

    trace!(desc = "Changing file path to optimal overview results file");
    // Write out to file
    file.set_file_path(conf.filepaths.results.optimal.clone());

    log_match_panic(
        file.clear_contents(),
        "Cleared file contents",
        "Failed to clear file contents",
    );

    trace!(desc = "Writing overview to file");
    // TODO: Use traits from types.rs
    log_match_panic(
        write_markdown(&mut file, &optimal_overview),
        "Wrote table to optimal_overview",
        "Failed to write table to optimal_overview",
    );

    trace!(desc = "Creating recipe lookups");

    // Get top n from the optimal overview
    let mut recipe_lookup_list: Vec<(String, Vec<Vec<String>>)> =
        optimal_overview
            .iter()
            // TODO: Make config load a usize not u32 for top n
            .take(conf.display.lookup.top.try_into().expect(
                "Number of values to take from top of optimal overview exceeds usize limit",
            ))
            .filter_map(|row| {
                let recipe_s = row[0].clone();
                let x = price_handle.recipe_list.get_recipe(&recipe_s)?;
                let specific_lookup = price_handle.recipe_lookup_from_recipe(x)?;
                Some((recipe_s, specific_lookup))
            })
            .collect();

    let recipe_lookup_list_specific: Vec<_> = conf
        .display
        .lookup
        .specific
        .clone()
        .into_iter()
        .filter_map(|recipe_s| {
            let x = price_handle.recipe_list.get_recipe(&recipe_s)?;
            let specific_lookup = price_handle.recipe_lookup_from_recipe(x)?;
            Some((recipe_s, specific_lookup))
        })
        .collect();

    recipe_lookup_list.extend(recipe_lookup_list_specific);

    trace!(desc = "Changing file path to recipe lookup results file");
    // Write out to file
    file.set_file_path(conf.filepaths.results.lookup.clone());

    // Clear file contents then append since loop
    log_match_panic(
        file.clear_contents(),
        "Cleared file contents",
        "Failed to clear file contents",
    );

    file = file.set_append(true);

    trace!(desc = "Writing detailed recipe lookups to file");
    for (recipe_name, recipe_lookup) in recipe_lookup_list {
        // TODO: Error message
        log_match_panic(
            write_recipe_lookup(&mut file, &recipe_name, recipe_lookup),
            &format!("Wrote recipe lookup for {recipe_name}"),
            &format!("Failed to write recipe lookup for {recipe_name}"),
        );
    }
}

fn request_new_prices_from_api(api_settings: &config::Api, file: &mut FileIO) {
    let api = Api::new(api_settings);
    let price_data = api.request_item_prices();

    // TODO: Should this be fatal?
    if let Err(e) = file.clear_contents() {
        warn!(desc = "Failed to clear file contents.", error = ?e);
    }

    log_match_panic(
        file.write_serialized(&price_data),
        "Write success.",
        "Failed to write to file.",
    );
}

// TODO: Takes in a nested iterator
#[instrument(level = "trace", skip(file, data))]
fn write_markdown(file: &mut FileIO, data: &osrs_gph::prices::prices::Table) -> io::Result<()> {
    // Open the underlying file handle
    let mut file = file.open_file().expect("Failed to access inner file");

    let num_cols = ROW_HEADERS.len();

    // Collect headers and rows into one iterable to calculate widths
    let mut col_widths = vec![0; num_cols];

    // Check headers lengths
    for (i, header) in ROW_HEADERS.iter().enumerate() {
        col_widths[i] = col_widths[i].max(header.len());
    }

    // Check all data rows lengths
    for row in data {
        for (i, cell) in row.iter().enumerate() {
            col_widths[i] = col_widths[i].max(cell.len());
        }
    }

    // Format a row with custom alignment rules
    fn format_row(row: &[String], col_widths: &[usize]) -> String {
        let num_cols = col_widths.len();

        let cells = row.iter().enumerate().map(|(i, cell)| {
            let width = col_widths[i];
            if i == 0 {
                // Left align first two columns
                format!("{cell:<width$}")
            } else if i >= num_cols - 2 {
                // Center align last two columns
                center_align(cell, width)
            } else {
                // Right align others
                format!("{cell:>width$}")
            }
        });

        format!("| {} |", cells.collect::<Vec<_>>().join(" | "))
    }

    // Write header row
    let header_row: Vec<String> = ROW_HEADERS.iter().map(|&s| s.to_string()).collect();
    writeln!(file, "{}", format_row(&header_row, &col_widths))?;

    // Write separator row
    let separator_cells = col_widths.iter().map(|w| "-".repeat(*w.max(&3)));
    writeln!(
        file,
        "| {} |",
        separator_cells.collect::<Vec<_>>().join(" | ")
    )?;

    // Write data rows
    for row in data {
        writeln!(file, "{}", format_row(row, &col_widths))?;
    }

    Ok(())
}

fn write_recipe_lookup(
    writer: &mut FileIO,
    recipe_name: &str,
    recipe_lookup: Vec<Vec<String>>,
) -> io::Result<()> {
    // Title
    writeln!(writer, "{recipe_name}\n")?;

    // Table
    let (rlstr, max_line_length) = markdown_table(recipe_lookup);

    write!(writer, "{rlstr}")?; // Already has newline

    // Buffer line
    write!(writer, "\n{}\n\n", "#".repeat(max_line_length))?;

    Ok(())
}

// Returns table as a string, and maximum line length
fn markdown_table(rows: Vec<Vec<String>>) -> (String, usize) {
    if rows.is_empty() {
        return (String::new(), 0);
    }

    let num_cols = rows.iter().map(std::vec::Vec::len).max().unwrap_or(0);

    let padded_rows: Vec<Vec<String>> = rows
        .into_iter()
        .map(|mut row| {
            row.resize(num_cols, String::new());
            row
        })
        .collect();

    let mut col_widths = vec![0; num_cols];
    for row in &padded_rows {
        for (i, cell) in row.iter().enumerate() {
            col_widths[i] = col_widths[i].max(cell.len());
        }
    }

    let mut output = String::new();
    let mut max_len = 0;

    for (i, row) in padded_rows.iter().enumerate() {
        let line = row
            .iter()
            .enumerate()
            .map(|(col_idx, cell)| {
                let width = col_widths[col_idx];
                if col_idx < 2 {
                    // Left align first two cols
                    format!("{cell:<width$}")
                } else if col_idx >= num_cols - 2 {
                    // Center align last two cols
                    center_align(cell, width)
                } else {
                    // Right align others
                    format!("{cell:>width$}")
                }
            })
            .collect::<Vec<_>>()
            .join(" | ");

        // Update max line length
        let full_line = format!("| {line} |");
        max_len = max_len.max(full_line.len());

        output.push_str(&full_line);
        output.push('\n');

        if i == 0 {
            let separator = col_widths
                .iter()
                .map(|w| "-".repeat(*w.max(&3)))
                .collect::<Vec<_>>()
                .join(" | ");
            let sep_line = format!("| {separator} |");
            max_len = max_len.max(sep_line.len());
            output.push_str(&sep_line);
            output.push('\n');
        }
    }

    (output, max_len)
}

fn center_align(s: &str, width: usize) -> String {
    if width <= s.len() {
        s.to_string()
    } else {
        let total_padding = width - s.len();
        let left_pad = total_padding / 2;
        let right_pad = total_padding - left_pad;
        format!("{}{}{}", " ".repeat(left_pad), s, " ".repeat(right_pad))
    }
}
