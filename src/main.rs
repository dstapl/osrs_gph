/// TODO: 2025-06-16 Something weird is happening with the log...
/// Unless the file is manually cleared, the contents will still remain

use std::{collections::HashMap, fs::File};

use osrs_gph::{api::{self, Api}, config, file_io::{FileIO, FileOptions}, helpers::Input, item_search::recipes::RecipeBook, log_match_panic, log_panic, prices::prices::PriceHandle, types::ROW_HEADERS};
use tracing::{debug, error, info, instrument, span, trace, warn, Level};


use std::io::Write;
use itertools::Itertools; // For iterator Join
// fn make_subscriber(filepaths: &config::FilePaths, log_level: Level) -> impl tracing::Subscriber {
//     let log_file_options = osrs_gph::file_io::FileOptions::new(false, true, true);
//     // Cloning because "borrowed data leaves the function"
//     let log_file = osrs_gph::file_io::FileIO::new(filepaths.log_file.clone(), log_file_options);
//
//     let subscriber = tracing_subscriber::fmt()
//         .with_writer(std::sync::Mutex::new(log_file))
//         .finish();
//
//     subscriber
// }


fn main() {
    let conf: config::Config = config::load_config("config.yaml");

    // Level:: ERROR, INFO, TRACE
    // Span levels are akin to the event levels: 
    //     too high and will revert to default guard instead of the span
    const LOG_LEVEL: Level = Level::TRACE;
    let subscriber = osrs_gph::make_subscriber(conf.filepaths.main_log_file.clone(), LOG_LEVEL);

    let _crateguard = tracing::subscriber::set_default(subscriber);
    let span = span!(LOG_LEVEL, "main");
    let guard = span.enter();

    trace!(desc = "Loaded config and created subscriber to log file.");

    // Initialise with price data file path
    let mut file = FileIO::new(conf.filepaths.price_data.clone(),
        FileOptions::new(true, true, true)
    );

    // api.set_timespan(Timespan::Latest)
    // let api: api::Api = api::Api::new(&conf.api);
    // let res = api.get_item_prices();

    trace!(desc = "Taking user input...");
    let inp = String::new().input("1. API Refresh Data\n2. Load previous Data\n");
    trace!(raw_input = %inp);

    let choice = inp.trim_end();
    trace!(choice = %choice);


    // Referesh API prices
    match choice {
        "1" => {info!(desc = "Retrieving prices from API.");
            request_new_prices_from_api(&conf.api, &mut file);
        },
        "2" => {info!(desc = "Loading previous data instead.");},
        _ => log_panic("Bad choice", choice)
    };


    // // Create item search
    // let (mut item_search_s, ignore_items) =
    //     create_item_search(&logger, &mut price_data_io, &id_to_name, &name_to_id, &config);

    let mut item_search = osrs_gph::item_search::item_search::ItemSearch::new(
       HashMap::new(), // Empty items list
       conf.filepaths.clone(),
       conf.api,
    );

    // Populate with items (from_file)
    let item_prices = item_search.get_item_prices(true);
    item_search.update_item_prices(item_prices);

    // dbg!(&item_search.items);
    // Get ignored items from the config
    let ignore_items: Vec<String> = conf.profit.ignore_items.clone();
    
    // Remove items contained in ignore_items
    item_search.ignore_items(&ignore_items);


    // dbg!(&item_search.items);

    // Load in recipes
    let mut recipe_list = RecipeBook::new(HashMap::new());
    recipe_list.load_default_recipes(conf.filepaths.lookup_data.recipes);

    trace!(desc = "Creating price handle...");
    let price_handle = PriceHandle::new(
        item_search, 
        recipe_list, 
        conf.profit.coins,
        conf.profit.percent_margin
    );


    trace!(desc = "Computing weights for pareto sort...");
    let weights = osrs_gph::prices::pareto_sort::custom_types::compute_weights(
        conf.profit.coins,
        conf.profit.weights
    );

    trace!(desc = "Creating all recipe overview");
    let optimal_overview = price_handle.all_recipe_overview(
        &weights,
        &conf.display,
    );
    assert!(!optimal_overview.is_empty());
    
    trace!(desc = "Changing file path to optimal overview results file");
    // Write out to file
    file.set_file_path(conf.filepaths.results.optimal.clone());


    trace!(desc = "Writing overview to file");
    // TODO: Use traits from types.rs
    write_markdown(&mut file, optimal_overview);
    // //
    // // optimal_overview.set_format(*FORMAT_MARKDOWN);
    // optimal_overview.set_titles(
    //     Row::new(
    //         vec![
    //         Cell::new_align("Method", LEFT_ALIGN),
    //         Cell::new_align("Loss/Gain", RIGHT_ALIGN),
    //         Cell::new_align("Total Loss/Gain", RIGHT_ALIGN),
    //         Cell::new_align("Time (h)", RIGHT_ALIGN),
    //         Cell::new_align("GP/h", RIGHT_ALIGN)
    //         ]
    //     )
    // );
    //
    // (logger, results_fps.clone(), optimal_overview)
    //

}

fn request_new_prices_from_api(api_settings: &config::Api, file: &mut FileIO) {
    let api = Api::new(api_settings);
    let price_data = api.request_item_prices();


    // TODO: Should this be fatal?
    if let Err(e) = file.clear_contents() {
        warn!(desc = "Failed to clear file contents.", error = ?e);
    }

    log_match_panic(file.write_serialized(&price_data),
        "Write success.",
        "Failed to write to file."
    )
}


// TODO: Takes in a nested iterator
#[instrument(level = "trace", skip(file, data))]
fn write_markdown(file: &mut FileIO, data: osrs_gph::prices::prices::Table) {
    // Write header
    let mut file: File = file.open_file().expect("Failed to access inner file");
    writeln!(file, "| {} |", ROW_HEADERS.join(" | "));

    // Write separator
    let header_sep_line = ROW_HEADERS.iter()
        .map(|_| "---")
        .collect::<Vec<_>>()
        .join("|");

    writeln!(file, "|{}|", header_sep_line);

    // Write rows
    for row in data.into_iter() {
        writeln!(file, "|{}|", row.into_iter().join(" | "));
    }
}
