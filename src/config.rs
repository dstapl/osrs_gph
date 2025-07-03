use std::{collections::HashMap, fs::File};

//use serde::de::Deserialize;
use serde::Deserialize;

#[derive(Deserialize, Debug, Default)]
/// Define config type
pub struct Config {
    pub title: String,
    pub api: Api,
    pub filepaths: FilePaths,
    pub profit: Profit,
    pub display: Display,
    pub levels: Levels,
}

#[derive(Deserialize, Debug, Default, Clone)]
pub enum TimeSpan {
    #[default]
    #[serde(rename = "latest")]
    Latest,
    #[serde(rename = "5m")]
    FiveMinute,
    #[serde(rename = "1h")]
    OneHour,
    // TODO: Extend to 6h(our), 24h(our)? This is only for specific item lookup
}

#[derive(Deserialize, Debug)]
pub struct Api {
    pub url: String,
    pub timespan: TimeSpan,
    pub auth_headers: HashMap<String, String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct LookupDataPaths {
    pub id_to_name: String,
    pub name_to_id: String,
    pub api_mapping: String,
    pub recipes: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ResultsPaths {
    pub optimal: String,
    pub lookup: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct FilePaths {
    pub price_data: String,
    pub lookup_data: LookupDataPaths,
    pub results: ResultsPaths,
    pub main_log_file: String,
    pub bin_log_file: String,
}

#[derive(Deserialize, Debug)]
pub struct Weights {
    pub margin: f32,
    pub time: f32,
    pub gph: f32,
}

#[derive(Deserialize, Debug)]
pub struct Profit {
    #[serde(deserialize_with = "deserialize_underscored_integer")]
    pub coins: i32,
    pub percent_margin: f32,
    pub weights: Weights,
    pub ignore_items: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct LookupOptions {
    pub top: u32,
    pub specific: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct Display {
    pub number: u32,
    pub lookup: LookupOptions,
    pub must_profit: bool,
    pub show_hidden: bool,
    pub reverse: bool,
}

#[derive(Debug)]
pub struct Levels {
    pub levels: HashMap<String, u32>,
    pub total_level: u32,
    // If a skill is marked as recommended
    //      should this level limit be encforced?
    pub strict_recommended: bool, 

}

impl Levels {
    fn new(levels: HashMap<String, u32>, strict_recommended: bool) -> Self {
        let mut levels = levels;
        let total_level = Self::_init_calc_total_level(&levels);

        levels.insert("total level".to_string(), total_level);
        Levels { levels , total_level, strict_recommended }
    }

    #[allow(dead_code)]
    fn calc_total_level(&self) -> u32 {
        let level_sum = Self::_init_calc_total_level(&self.levels);
        
        let curr_total_level: u32 = self.levels.get("total level")
            .unwrap_or_else(|| &0).to_owned();

        let curr_quest_points: u32 = self.levels.get("quest points")
            .unwrap_or_else(|| &0).to_owned();
        level_sum  - curr_total_level - curr_quest_points

    }
    fn _init_calc_total_level(levels: &HashMap<String, u32>) -> u32 {
        let level_sum: u32 = levels // Includes total_level
            .values().map(|&x| 
                u32::from(x)
            ).sum::<u32>().into();

        level_sum
    }
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
            id_to_name: "lookup_data/id_to_name.yaml".to_string(),
            name_to_id: "lookup_data/name_to_id.yaml".to_string(),
            recipes: "lookup_data/recipes.yaml".to_string(),
            // External file
            api_mapping: "lookup_data/mapping.json".to_string(),
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
            main_log_file: "runtime.log".to_string(),
            bin_log_file: "generators.log".to_string(),
        }
    }
}

impl Default for Weights {
    fn default() -> Self {
        Self {
            margin: 1e-2,
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

impl Default for Levels {
    fn default() -> Self {
        let mut levels = HashMap::with_capacity(23);
        levels.insert("hitpoints".to_string(), 10);
        levels.insert("attack".to_string(), 1);
        levels.insert("defence".to_string(), 1);
        levels.insert("strength".to_string(), 1);
        levels.insert("ranged".to_string(), 1);
        levels.insert("prayer".to_string(), 1);
        levels.insert("magic".to_string(), 1);
        levels.insert("cooking".to_string(), 1);
        levels.insert("woodcutting".to_string(), 1);
        levels.insert("fletching".to_string(), 1);
        levels.insert("fishing".to_string(), 1);
        levels.insert("firemaking".to_string(), 1);
        levels.insert("crafting".to_string(), 1);
        levels.insert("smithing".to_string(), 1);
        levels.insert("mining".to_string(), 1);
        levels.insert("herblore".to_string(), 1);
        levels.insert("agility".to_string(), 1);
        levels.insert("thieving".to_string(), 1);
        levels.insert("slayer".to_string(), 1);
        levels.insert("farming".to_string(), 1);
        levels.insert("runecraft".to_string(), 1);
        levels.insert("hunter".to_string(), 1);
        levels.insert("construction".to_string(), 1);
        levels.insert("quest points".to_string(), 0);
        //let total_level: u32 = Self::_init_calc_total_level(&levels);
        //levels.insert("total level".to_string(), total_level);
        Levels::new(levels, false)
    }
}

#[derive(Debug)]
pub enum ConfigError {
    FileError(std::io::Error),
    DeserializeError(serde_yaml_ng::Error),
}

impl From<std::io::Error> for ConfigError {
    fn from(value: std::io::Error) -> Self {
        ConfigError::FileError(value)
    }
}
impl From<serde_yaml_ng::Error> for ConfigError {
    fn from(value: serde_yaml_ng::Error) -> Self {
        ConfigError::DeserializeError(value)
    }
}

pub fn load_config<P: AsRef<std::path::Path>>(filepath: P) -> Config {
    File::open(&filepath)
        .map_err(ConfigError::FileError)
        .and_then(|file| serde_yaml_ng::from_reader(file).map_err(ConfigError::DeserializeError))
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


use serde::{de::Visitor, Deserializer};
use std::fmt;
impl<'de> Deserialize<'de> for Levels {
    // #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct LevelsVisitor;

        impl<'de> Visitor<'de> for LevelsVisitor {
            type Value = Levels;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                // formatter.write_str("four fields containting values/nones: i32,u32,i32,u32.")
                formatter.write_str("23 fields of u32 corresponding to the level in each OSRS skill.")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                // TODO: Is there a way to write this using different visitors?

                // First value will be `option: strict_recommended: bool`
                let (_, options_map) = map.next_entry::<String, HashMap<String,bool>>()?
                    .expect("Failed to deserialize Levels.options");
                let strict_recommended: bool = *options_map.get("strict_recommended")
                    .expect("Failed to find strict_recommended in config file");


                let (_, skill_map) = map.next_entry::<String, HashMap<String, u32>>()?
                    .expect("Failed to deserialize levels in config file");

                let levels = skill_map.into_iter()
                    .map(|(key, value)| (key.to_lowercase(), value))
                    .collect::<HashMap<String, u32>>();

                Ok(Levels::new(levels, strict_recommended))
            }
        }
        deserializer.deserialize_map(LevelsVisitor)
    }
}
