use std::{collections::HashMap, fs::File};

//use serde::de::Deserialize;
use serde::Deserialize;

#[derive(Deserialize, Debug, Default)]
/// Define config type
pub struct Config {
    title: String,
    api: Api,
    filepaths: FilePaths,
    profit: Profit,
    display: Display,
}

#[derive(Deserialize, Debug, Default)]
enum TimeSpan {
    #[default]
    #[serde(rename = "latest")]
    Latest,
    #[serde(rename = "5m")]
    FiveMinute,
    #[serde(rename = "1h")]
    OneHour,
}

#[derive(Deserialize, Debug)]
pub struct Api {
    url: String,
    timespan: TimeSpan,
    auth_headers: HashMap<String, String>,
}

#[derive(Deserialize, Debug)]
struct LookupDataPaths {
    id_to_name: String,
    name_to_id: String,
    api_mapping: String,
    recipes: String,
}

#[derive(Deserialize, Debug)]
struct ResultsPaths {
    optimal: String,
    lookup: String,
}

#[derive(Deserialize, Debug)]
struct FilePaths {
    price_data: String,
    lookup_data: LookupDataPaths,
    results: ResultsPaths,
    log_file: String,
}

#[derive(Deserialize, Debug)]
struct Weights {
    margin_to_time: f32,
    time: f32,
    gph: f32,
}

#[derive(Deserialize, Debug)]
struct Profit {
    #[serde(deserialize_with = "deserialize_underscored_integer")]
    coins: i32,
    percent_margin: f32,
    weights: Weights,
    ignore_items: Vec<String>,
}

#[derive(Deserialize, Debug)]
struct LookupOptions {
    top: u32,
    specific: Vec<String>,
}

#[derive(Deserialize, Debug)]
struct Display {
    number: u32,
    lookup: LookupOptions,
    must_profit: bool,
    show_hidden: bool,
    reverse: bool,
}

impl Default for Api {
    fn default() -> Self {
        let auth_headers = vec![("User-Agent", "profit_margins - @blamblamdan")];
        Self {
            url: "https://prices.runescape.wiki/api/v1/osrs".to_string(),
            timespan: TimeSpan::default(),
            auth_headers: HashMap::<String, String>::from_iter(
                auth_headers
                    .into_iter()
                    .map(|(k, v)| (k.to_string(), v.to_string())),
            ),
        }
    }
}

impl Default for LookupDataPaths {
    fn default() -> Self {
        Self {
            id_to_name: "lookup_data/id_to_name.json".to_string(),
            name_to_id: "lookup_data/name_to_id.json".to_string(),
            api_mapping: "lookup_data/mapping.json".to_string(),
            recipes: "lookup_data/recipes.json".to_string(),
        }
    }
}
impl Default for ResultsPaths {
    fn default() -> Self {
        Self {
            optimal: "results/optimal_overview.md".to_string(),
            lookup: "results/recipe_lookup.md".to_string(),
        }
    }
}
impl Default for FilePaths {
    fn default() -> Self {
        Self {
            price_data: "api_data/price_data.json".to_string(),
            lookup_data: LookupDataPaths::default(),
            results: ResultsPaths::default(),
            log_file: "runtime.log".to_string(),
        }
    }
}

impl Default for Weights {
    fn default() -> Self {
        Self {
            margin_to_time: 1e-2,
            time: -2.0,
            gph: 1e-5,
        }
    }
}
impl Default for Profit {
    fn default() -> Self {
        let ignore_items: Vec<&str> = vec![];
        Self {
            coins: 2_000_000,
            percent_margin: 2.5,
            weights: Weights::default(),
            ignore_items: ignore_items.iter().map(ToString::to_string).collect(),
        }
    }
}

impl Default for LookupOptions {
    fn default() -> Self {
        Self {
            top: 3,
            specific: Vec::new(),
        }
    }
}
impl Default for Display {
    fn default() -> Self {
        Self {
            number: 0,
            lookup: LookupOptions::default(),
            must_profit: true,
            show_hidden: false,
            reverse: true,
        }
    }
}

#[derive(Debug)]
pub enum ConfigError {
    FileError(std::io::Error),
    DeserializeError(serde_yml::Error),
}

impl From<std::io::Error> for ConfigError {
    fn from(value: std::io::Error) -> Self {
        ConfigError::FileError(value)
    }
}
impl From<serde_yml::Error> for ConfigError {
    fn from(value: serde_yml::Error) -> Self {
        ConfigError::DeserializeError(value)
    }
}

pub fn load_config<P: AsRef<std::path::Path>>(filepath: P) -> Config {
    File::open(&filepath)
        .map_err(ConfigError::FileError)
        .and_then(|file| serde_yml::from_reader(file).map_err(ConfigError::DeserializeError))
        .unwrap_or_else(|e| panic!("{e:?}"))
}


/// To parse underscored integer representaions
fn deserialize_underscored_integer<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::de::Deserializer<'de>,
    T: std::str::FromStr,
{
    // First, deserialize the value as a string (which might fail...)
    let mut s: String = serde::de::Deserialize::deserialize(deserializer)?;
    
    s.retain(char::is_numeric);

    s.parse().map_err(|_: <T as std::str::FromStr>::Err| {
        serde::de::Error::custom("string does not represent an integer")
    })
}