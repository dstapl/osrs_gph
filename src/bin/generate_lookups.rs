use osrs_gph::api::MappingItem;
use osrs_gph::config::{self, load_config};
use osrs_gph::file_io::{self, SerChoice};
use std::collections::HashMap;

fn main() {
    let config: config::Config = load_config("config.yaml");

    let id_to_name_str: &str = &config.filepaths.lookup_data.id_to_name;
    let name_to_id_str: &str = &config.filepaths.lookup_data.name_to_id;

    let mapping_path_str: &str = &config.filepaths.lookup_data.api_mapping;



    let mut mapping_fio = file_io::FileIO::new(
            mapping_path_str,
            [true, true, true]
        );

    let mapping: Vec<MappingItem> = mapping_fio.read(SerChoice::JSON).expect("Failed to parse mapping file");

    let mut id_to_name = HashMap::<String, String>::with_capacity(mapping.len());
    let mut name_to_id = HashMap::<String, String>::with_capacity(mapping.len());

    for item in mapping {
        let item_id = item.id.to_string();
        let item_name = item.name.to_string();

        id_to_name.insert(item_id.clone(), item_name.clone());
        name_to_id.insert(item_name, item_id);
    }

    mapping_fio.set_file_path(id_to_name_str);
    let _ = mapping_fio.clear_contents();
    let _ = mapping_fio.write(&id_to_name);

    let _ = mapping_fio.set_file_path(name_to_id_str);
    let _ = mapping_fio.clear_contents();
    let _ = mapping_fio.write(&name_to_id);
}
