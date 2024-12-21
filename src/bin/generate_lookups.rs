use osrs_gph::config::{self, load_config};
use serde::Deserialize;
use std::collections::HashMap;

fn main() {
    let config: config::Config = load_config("config.yaml");

    dbg!("config = ", &config);
    //config
}
