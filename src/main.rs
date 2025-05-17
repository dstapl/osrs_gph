use osrs_gph::{config, api};
use tracing::{debug, info, error, span, trace, warn, Level};


fn make_subscriber(filepaths: &config::FilePaths, log_level: Level) -> impl tracing::Subscriber {
    let log_file_options = osrs_gph::file_io::FileOptions::new(false, true, true);
    // Cloning because "borrowed data leaves the function"
    let log_file = osrs_gph::file_io::FileIO::new(filepaths.log_file.clone(), log_file_options);

    let subscriber = tracing_subscriber::fmt()
        .with_writer(std::sync::Mutex::new(log_file))
        .finish();

    subscriber
}


fn main() {
    let conf: config::Config = config::load_config("config.yaml");

    // Level:: ERROR, INFO, TRACE
    // Span levels are akin to the event levels: 
    //     too high and will revert to default guard instead of the span
    const LOG_LEVEL: Level = Level::TRACE;
    let subscriber = make_subscriber(&conf.filepaths, LOG_LEVEL);
   
    let _crateguard = tracing::subscriber::set_default(subscriber);
    let span = span!(LOG_LEVEL, "main").entered();
    trace!("Loaded config and created subscriber to log file.");


    let api: api::Api = api::Api::new(&conf.api);
    let res = api.request();

    let span = span.exit();

}

