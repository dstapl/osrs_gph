//! TODO: 2025-06-16 Something weird is happening with the log...
//! Unless the file is manually cleared, the contents will still remain

use std::collections::HashMap;

use osrs_gph::{
    api::Api, check_items_exists, config::{self, OverviewFilter}, file_io::{FileIO, FileOptions}, item_search::recipes::RecipeBook, log_match_panic, prices::prices::{PriceHandle, TimeType}, results_writer::markdown::{DetailedRecipeLookup, OptimalOverview}, types::{DetailedTable, ResultsTable, DETAILED_NUM_HEADERS, OVERVIEW_NUM_HEADERS}
};
use tracing::{info, span, trace, warn, Level};

use clap::{Parser, builder::ArgAction};

#[derive(Parser)]
#[command(about, long_about = None)]
struct Cli {
    /// Refresh prices or not
    #[clap(short = 'r', long, action = ArgAction::SetTrue)]
    refresh: bool,

    #[clap(short = 's', long = "show-hidden", action = ArgAction::SetTrue)]
    show_hidden: bool,

    // TODO: Change this to a value (number of hours to calc for)
    //  : no value supplied means one hour; 2 hours means 2 hours etc.

    /// Calculate the amount of money made in one hour
    /// Default behaviour: Calculate the recipes for the maximum number of hours with the given money
    #[clap(short = 'o', long = "one-hour", action = ArgAction::SetTrue)]
    number_hours: bool
}


fn main() {
    // Level:: ERROR, INFO, TRACE
    const LOG_LEVEL: Level = Level::TRACE;

    let mut conf: config::Config = config::load_config("config.yaml");

    // Span levels are akin to the event levels:
    //     too high and will revert to default guard instead of the span
    let subscriber = osrs_gph::make_subscriber(conf.filepaths.main_log_file.clone(), LOG_LEVEL);

    let _crateguard = tracing::subscriber::set_default(subscriber);
    let span = span!(LOG_LEVEL, "main");
    let _guard = span.enter();

    trace!(desc = "Loaded config and created subscriber to log file.");


    trace!(desc = "Parsing CLI arguments");
    let cli = Cli::parse();


    // Initialise with price data file path
    let mut file = FileIO::new(
        conf.filepaths.price_data.clone(),
        FileOptions::new(true, true, true),
    );


    trace!(desc = "Handling refresh flag");
    let choice: bool = cli.refresh;
    trace!(refresh = choice);

    // Referesh API prices
    if choice {
        let msg = "Retrieving prices from API.";
        info!(desc = msg);
        println!("{msg}");
        request_new_prices_from_api(&conf.api, &mut file);
    } else {
        let msg = "Loading previous data instead.";
        info!(desc = msg);
        println!("{msg}");
    }

    trace!(desc = "Handling show-hidden flag");
    let show_hidden: bool = cli.show_hidden;
    trace!(show_hidden = show_hidden);

    conf.display.filters[OverviewFilter::ShowHidden] = show_hidden;



    trace!(desc = "Handling max-hours flag");
    // Override config with new value
    conf.display.time_type = if cli.number_hours {
        TimeType::SingleHour
    } else {
        TimeType::MaxHours
    };
    let time_type = conf.display.time_type;
    trace!(time_type = ?conf.display.time_type);

    // Create item search
    let mut item_search = osrs_gph::item_search::item_search::ItemSearch::new(
        HashMap::new(), // Empty items list
        conf.filepaths.clone(),
        conf.api,
    );

    // Populate with items (from_file)
    let item_prices = item_search.get_item_prices(true);
    item_search.update_item_prices(item_prices);
    let mapping_path = conf.filepaths.lookup_data.api_mapping.clone();
    log_match_panic(
        item_search.update_item_extra_info(mapping_path),
        "Updated items with extra information",
        "Failed to update items with extra information",
    );

    trace!(desc = "After update_item_prices");
    // Check important items exist in memory
    check_items_exists(&item_search, &["Coins"]);

    // Get ignored items from the config
    let ignore_items: Vec<String> = conf.profit.ignore_items.clone();

    // Remove items contained in ignore_items
    item_search.ignore_items(&ignore_items);

    // Load in recipes
    let mut recipe_list = RecipeBook::new(HashMap::new());
    recipe_list.load_default_recipes(conf.filepaths.lookup_data.recipes);

    // Get ignored methods from the config
    let ignore_methods: Vec<String> = conf.profit.ignore_methods.clone();
    recipe_list.ignore_recipes(ignore_methods);

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
    let sort_by = conf.display.sort_by;
    let optimal_overview = price_handle.all_recipe_overview(&sort_by, &weights, &conf.display);
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
    // TODO: Possible to take reference to optimal_overview instead?
    let mut writer = OptimalOverview::new(optimal_overview.clone(), [0; OVERVIEW_NUM_HEADERS]);

    // TODO: Optimise into reduced/buffered calls?
    // Set append mode since all rows are written in separate calls
    file = file.set_append(true);

    log_match_panic(
        writer.write_table(&mut file),
        "Wrote table to optimal_overview",
        "Failed to write table to optimal_overview",
    );

    file = file.set_append(false);


    trace!(desc = "Creating recipe lookups");

    // Get top n from the optimal overview
    let mut recipe_lookup_list: Vec<DetailedTable> =
        optimal_overview
            .iter()
            // TODO: Make config load a usize not u32 for top n
            .take(conf.display.lookup.top.try_into().expect(
                "Number of values to take from top of optimal overview exceeds usize limit",
            ))
            .filter_map(|row| {
                // TODO: This won't include any rows that have a name modifier
                // E.g. if `*` is appended to the name due to filters
                let recipe_s = row.name.clone();
                let x = price_handle.recipe_list.get_recipe(&recipe_s)?;
                let specific_lookup = price_handle.recipe_lookup_from_recipe(x, time_type)?;
                Some(specific_lookup)
            })
            .collect();

    let recipe_lookup_list_specific: Vec<DetailedTable> = conf
        .display
        .lookup
        .specific
        .clone()
        .into_iter()
        .filter_map(|recipe_s| {
            let x = price_handle.recipe_list.get_recipe(&recipe_s)?;
            let specific_lookup = price_handle.recipe_lookup_from_recipe(x, time_type)?;
            Some(specific_lookup)
        })
        .collect();

    recipe_lookup_list.extend(recipe_lookup_list_specific);

    // Filter duplicates
    recipe_lookup_list.sort_by_key(|e| e.overview.name.clone());
    recipe_lookup_list.dedup_by_key(|e| e.overview.name.clone());


    trace!(desc = "Creating DetailedRecipeLookup struct");
    let mut writer = DetailedRecipeLookup::new(
        conf.profit.coins,
        recipe_lookup_list,
        [0;DETAILED_NUM_HEADERS]
    );
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

    log_match_panic(
        writer.write_all_tables(&mut file),
        "Wrote all recipe lookups to file",
        "Failed to write all recipe tables",
    );
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

