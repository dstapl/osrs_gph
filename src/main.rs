use osrs_gph::{api, config, helpers::Input};
use tracing::{debug, info, error, span, trace, warn, Level};


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
    let subscriber = osrs_gph::make_subscriber(conf.filepaths.main_log_file, LOG_LEVEL);

    let _crateguard = tracing::subscriber::set_default(subscriber);
    let span = span!(LOG_LEVEL, "main");
    let guard = span.enter();

    trace!(desc = "Loaded config and created subscriber to log file.");


    let inp = String::new().input("Enter a value");


    println!("Printing input: {}", &inp);
    // debug!("Recieved input: {}", inp);
    debug!(input = ?inp);



    // api.set_timespan(Timespan::Latest)
    // let api: api::Api = api::Api::new(&conf.api);
    // let res = api.get_item_prices();

}

