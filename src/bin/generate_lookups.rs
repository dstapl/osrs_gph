use std::collections::HashMap;

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
    let mut mapping_text: String = client
        .get(config.api.url + "/mapping")
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .send().expect("Failed to send API request")
        // TODO: Hopefully this won't get too big to fit in memory...
        .text().expect("Failed to parse text of response");

    // Write Response to file
    let mapping_path_str: String = config.filepaths.lookup_data.api_mapping;

    trace!(desc = "Creating mapping_fio");
    let mut mapping_fio =
        file_io::FileIO::new(mapping_path_str.clone(), FileOptions::new(true, true, true));


    // Need to convert json to a common `Value` that serde_yaml_ng can write out
    let mut mapping_value: Vec<serde_json::Value>= serde_json::from_str(&mapping_text)
        .expect("Failed to parse json response into a value");
    

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

    let special_items: Vec<serde_json::Value> = serde_json::from_str(
        &serde_json::ser::to_string(
            &vec![coins]
        )
        .expect("Failed to convert special items into json")
    ).expect("Failed to convert special items json str into json value");
        

    // Concatenate
    mapping_value.extend(special_items);

    // Convert to YAML
    trace!(desc = "Re-/serialising mapping into YAML");
    mapping_text = serde_yaml_ng::to_string(&mapping_value)
        .expect("Failed to convert JSON into YAML");
    let mapping: Vec<MappingItem> = serde_yaml_ng::from_str(&mapping_text).expect("Failed to serialise mapping_text into YAML");


    // Format and write to mapping file
    let mapping_hashmap: HashMap<&String, &MappingItem> = 
        mapping.iter()
        .map(|item| (&item.name, item))
        .collect::<HashMap<_,_>>();
    trace!(desc = "Writing mapping to file");
    log_match_panic(
        mapping_fio.write_serialized(&mapping_hashmap),
        &format!("Reading mapping file {mapping_path_str}"),
        &format!("Failed to parse mapping {mapping_path_str}"),
    );
    

    // Split mapping into id_to_name and name_to_id
    let id_to_name_str: String = config.filepaths.lookup_data.id_to_name;
    let name_to_id_str: String = config.filepaths.lookup_data.name_to_id;

    let mut id_to_name = HashMap::<String, String>::with_capacity(mapping.len());
    trace!(desc = "Initialised id_to_name HashMap");

    let mut name_to_id = HashMap::<String, String>::with_capacity(mapping.len());
    trace!(desc = "Initialised name_to_id HashMap");

    debug!(desc = "Inserting values into mappings...");
    for item in mapping {
        let item_id = item.id.to_string();
        let item_name = item.name.to_string();

        let mut val = id_to_name.insert(item_id.clone(), item_name.clone());
        trace!(mapping = "id_to_name", value = ?val);

        val = name_to_id.insert(item_name, item_id);
        trace!(mapping = "name_to_id", value = ?val);
    }

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
