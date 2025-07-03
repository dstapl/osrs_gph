use osrs_gph::api::MappingItem;
use osrs_gph::config::{self, load_config};
use osrs_gph::file_io::{self, FileOptions, SerChoice};
use osrs_gph::log_match_panic;
use std::collections::HashMap;


use tracing::{debug, info, error, span, trace, warn, Level};

fn main() {
    let config_file_name = "config.yaml";
    let config: config::Config = load_config(config_file_name);

    const LOG_LEVEL: Level = Level::TRACE;
    let subscriber = osrs_gph::make_subscriber(config.filepaths.bin_log_file.clone(), LOG_LEVEL);

    let _crateguard = tracing::subscriber::set_default(subscriber);
    let _span = span!(LOG_LEVEL, "main").entered();

    trace!("Initialised logger: {config_file_name}");

    let id_to_name_str: String = config.filepaths.lookup_data.id_to_name;
    let name_to_id_str: String = config.filepaths.lookup_data.name_to_id;

    let mapping_path_str: String = config.filepaths.lookup_data.api_mapping;


    let mut mapping_fio = file_io::FileIO::new(
            mapping_path_str.clone(),
            FileOptions::new(true, true, true)
        );
    trace!(desc = "Created mapping_fio");

    let mapping: Vec<MappingItem> = log_match_panic(mapping_fio.read_serialized(SerChoice::JSON),
        &format!("Reading mapping file {}", mapping_path_str),
        &format!("Failed to parse mapping {}", mapping_path_str)
    );

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
    let _ = mapping_fio.set_file_path(id_to_name_str);

    let _ = log_match_panic(mapping_fio.clear_contents(),
        "Clearing file contents.",
        "Failed to clear file contents."
    );
    let _ = log_match_panic(mapping_fio.write_serialized(&id_to_name),
        "Writing serialised data to id_to_name file.",
        "Failed to write data."
    );

    trace!(desc = "Setting mapping_fio file path", value = %name_to_id_str);
    let _ = mapping_fio.set_file_path(name_to_id_str);

    let _ = log_match_panic(mapping_fio.clear_contents(),
        "Clearing file contents.",
        "Failed to clear file contents."
    );
    let _ = log_match_panic(mapping_fio.write_serialized(&name_to_id),
        "Writing serialised data to name_to_id file.",
        "Failed to write data."
    );
}
