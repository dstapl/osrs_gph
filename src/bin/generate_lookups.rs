use std::collections::HashMap;
use std::io::Write;

use osrs_gph::api::MappingItem;
use osrs_gph::config::{self, load_config};
use osrs_gph::file_io::{self, FileOptions};
use osrs_gph::log_match_panic;

use tracing::{debug, span, trace, Level};


fn main() {
    const LOG_LEVEL: Level = Level::TRACE;
    const USER_AGENT: &str =
        "Mozilla/5.0 (Macintosh; Intel Mac OS X x.y; rv:42.0) Gecko/20100101 Firefox/42.0";

    let config_file_name = "config.yaml";
    let config: config::Config = load_config(config_file_name);

    let subscriber = osrs_gph::make_subscriber(config.filepaths.bin_log_file.clone(), LOG_LEVEL);

    let _crateguard = tracing::subscriber::set_default(subscriber);
    let _span = span!(LOG_LEVEL, "main").entered();

    trace!(desc = "Initialised logger: {config_file_name}");

    trace!(desc = "Building reqwest client");
    // Get new mapping file
    let client = reqwest::blocking::Client::new();

    trace!(desc = "Sending API request");
    let mut mapping: Vec<MappingItem> = client
        .get(config.api.url + "/mapping")
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .send().expect("Failed to send API request")
        .json().expect("Failed to deserialize json");

    // Write Response to file
    let mapping_path_str: String = config.filepaths.lookup_data.api_mapping;

    trace!(desc = "Creating mapping_fio");
    let mut mapping_fio =
        file_io::FileIO::new(mapping_path_str.clone(), FileOptions::new(true, true, true));

    // Add special currencies into mapping
    let coins = MappingItem{
        members: false,
        name: "Coins".to_string(),
        examine: "Lovely money!".to_string(),
        id: 995, // Basic Coins
        value: Some(1),
        icon: "https://oldschool.runescape.wiki/images/thumb/Coins_detail.png/240px-Coins_detail.png".to_string(),
        limit: None,
        // highalch: None
        // lowalch: None,
        alchable: None
    };

    let special_items: Vec<MappingItem> = vec![
        coins
    ];
    mapping.extend(special_items);


    {
        // Format and write to mapping file
        let mapping_hashmap: HashMap<&String, &MappingItem> = 
            mapping.iter()
            .map(|item| (&item.name, item))
            .collect();

        trace!(desc = "Writing mapping to file");
        log_match_panic(
            mapping_fio.clear_contents(),
            "Clearing file contents.",
            "Failed to clear file contents.",
        );
        log_match_panic(
            mapping_fio.write_serialized(&mapping_hashmap),
            &format!("Reading mapping file {mapping_path_str}"),
            &format!("Failed to parse mapping {mapping_path_str}"),
        );
        
        // Force flush
        mapping_fio.flush().expect("Failed to flush mapping to disk");
    }

    // Split mapping into id_to_name and name_to_id
    let mut id_to_name = HashMap::<String, String>::with_capacity(mapping.len());
    trace!(desc = "Initialised id_to_name HashMap");

    let mut name_to_id = HashMap::<String, String>::with_capacity(mapping.len());
    trace!(desc = "Initialised name_to_id HashMap");

    debug!(desc = "Inserting values into mappings...");
    for item in mapping {
        let item_id = item.id.to_string();
        let item_name = item.name;

        id_to_name.insert(item_id.clone(), item_name.clone());
        name_to_id.insert(item_name, item_id);
    }


    // Write new mappings out to files
    let id_to_name_str: String = config.filepaths.lookup_data.id_to_name;
    trace!(desc = "Setting mapping_fio file path", value = %id_to_name_str);
    mapping_fio.set_file_path(id_to_name_str);

    log_match_panic(
        mapping_fio.clear_contents(),
        "Clearing file contents.",
        "Failed to clear file contents.",
    );
    log_match_panic(
        mapping_fio.write_serialized(&id_to_name),
        "Writing serialised data to id_to_name file.",
        "Failed to write data.",
    );

    let name_to_id_str: String = config.filepaths.lookup_data.name_to_id;
    trace!(desc = "Setting mapping_fio file path", value = %name_to_id_str);
    mapping_fio.set_file_path(name_to_id_str);

    log_match_panic(
        mapping_fio.clear_contents(),
        "Clearing file contents.",
        "Failed to clear file contents.",
    );
    log_match_panic(
        mapping_fio.write_serialized(&name_to_id),
        "Writing serialised data to name_to_id file.",
        "Failed to write data.",
    );
}
