use osrs_gph::config;

fn main() {
    println!("Hello, world!");
    let con = config::load_config("config.yaml");
    println!("{con:?}");
}
