//! Parsing items from [api](src/api.rs) data
//! TODO: Support for different modules (timespans) other than just latest
use super::data_types::latest::{self, PriceDataType, SPECIAL_ITEM_NAMES}; //::PriceDatum;

use tracing::{debug, instrument, warn};

use serde::Deserialize;
use std::{collections::HashMap, fmt::Debug, hash::Hash};

use crate::config::FilePaths;
use crate::file_io::{FileIO, FileOptions};
use crate::item_search::data_types::latest::PriceDatum;
use crate::{file_io, log_match_panic, log_panic};


#[derive(Debug, Deserialize, Clone)]
pub struct Item {
    pub name: String,    // TODO: Consider switching to &str if not needed.
    pub item_id: String, // i32
    pub item_prices: latest::PriceDatum,
}

// Required by HashMaps
impl PartialEq for Item {
    fn eq(&self, other: &Self) -> bool {
        self.item_id == other.item_id
    }
}
impl Eq for Item {}

impl Hash for Item {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.item_id.hash(state);
    }
}

impl Item {
    pub fn new(name: String, id: String, price_data: latest::PriceDatum) -> Self {
        Self {
            name,
            item_id: id,
            item_prices: price_data,
        }
    }

    pub fn invalid_data(&self) -> bool {
        // Ignore special items
        let reserved = SPECIAL_ITEM_NAMES
            .contains(&self.name.as_str());

        !reserved && self.item_prices.invalid_data()
    }

    pub fn price(&self, high_price: bool) -> Option<i32> {
        if high_price {
            self.item_prices.high
        } else {
            self.item_prices.low
        }
    }

    pub fn price_tuple(&self) -> HashMap<String, Option<i32>> {
        HashMap::from_iter([
            ("high".to_owned(), self.item_prices.high),
            ("low".to_owned(), self.item_prices.low),
        ])
    }
}

pub struct ItemSearch {
    // NOTE: **Handlers replaced by filenames**
    // pub price_data_fio: FileIO,
    // pub name_to_id_fio: FileIO,
    // pub id_to_name_fio: FileIO,
    pub items: HashMap<String, Item>,
    pub filepaths: FilePaths,
    // TODO: Better way to do api_config?
    pub api_config: crate::config::Api,

    // Populated from filepaths
    pub name_to_id: HashMap<String, String>,
    pub id_to_name: HashMap<String, String>,
}

impl ItemSearch {
    pub fn new(
        items: HashMap<String, Item>,
        filepaths: FilePaths,
        api_config: crate::config::Api,
    ) -> Self {
        // TODO: Populate name_to_id and id_to_name
        let mut intermediate = Self {
            items, // Using Item Name(String)=>Item(Object)
            filepaths,
            api_config,
            name_to_id: HashMap::new(),
            id_to_name: HashMap::new(),
        };

        intermediate.populate_lookups();

        intermediate
    }

    fn populate_lookups(&mut self) {
        // Create fileio
        let mut file = FileIO::new(
            self.filepaths.lookup_data.id_to_name.clone(),
            FileOptions::new(true, true, false), // Don't want to make new files
        );

        self.id_to_name = log_match_panic(
            file.read_serialized(file_io::SerChoice::YAML),
            "Reading id_to_name lookup data",
            "Failed to Deserialize id_to_name",
        );

        file.set_file_path(self.filepaths.lookup_data.name_to_id.clone());
        self.name_to_id = log_match_panic(
            file.read_serialized(file_io::SerChoice::YAML),
            "Reading name_to_id lookup data",
            "Failed to Deserialize name_to_id",
        );
    }

    pub fn name_from_id(&self, item_id: &String) -> Option<&String> {
        self.id_to_name.get(item_id)
    }

    pub fn id_from_name(&self, item_name: &String) -> Option<&String> {
        self.name_to_id.get(item_name)
    }

    pub fn item_by_name(&self, item_name: &String) -> Option<&Item> {
        self.items.get(item_name)
    }

    pub fn item_by_id(&self, item_id: &String) -> Option<&Item> {
        if let Some(item_name) = self.name_from_id(item_id) {
            self.item_by_name(item_name)
        } else {
            None
        }
    }


    /// Either from file (ideally) or from the api
    /// # Panics
    /// Will panic when the api response is empty
    pub fn get_item_prices(&mut self, from_file: bool) -> PriceDataType {
        let res = if from_file {
            log_match_panic(
                self.find_prices_from_file(),
                "Attempting to find prices from a stored mapping file.",
                "Failed to find prices from file. May not exist or malformed data.",
            )

        } else {
            let api = crate::api::Api::new(&self.api_config);
            api.request_item_prices()
        };

        assert!(!res.data.is_empty());

        res
    }

    /// Attempts to load item prices
    /// from a file defined in config
    fn find_prices_from_file(&mut self) -> Result<PriceDataType, std::io::Error> {
        // Get correct file name and try to load contents
        let mut price_io = crate::file_io::FileIO::new(
            self.filepaths.price_data.clone(),
            FileOptions::new(true, false, false),
        );

        price_io.read_serialized(file_io::SerChoice::YAML)
    }

    /// Removes items from the internal list.
    /// Returns number of items removed.
    #[instrument(level = "debug", skip(self))]
    pub fn ignore_items(&mut self, item_name_list: &Vec<String>) -> i32 {
        debug!(desc = "Removing ignored items...");
        match item_name_list
            .iter()
            .filter_map(|x| self.items.remove(x))
            .count()
            .try_into()
        {
            Ok(n) => {
                debug!(desc = "Removed ignored items.", count = %n);
                n
            }
            Err(e) => log_panic("Number of ignored items is too big.", e),
        }
    }


    fn add_special_values(&mut self) {
        // Add special values
        const COIN_VALUE: i32 = 1;
        const START_TIME: i32 = 0;
        let coins_prices = PriceDatum{
            high: Some(COIN_VALUE),
            low: Some(COIN_VALUE),
            // Start of time
            high_time: Some(START_TIME),
            low_time: Some(START_TIME),
        };

        let coins_datum = self.item_from_id_price("Coins".to_string(), coins_prices)
            .expect("No ID found for `Coins`");
        self.items.insert("Coins".to_string(), coins_datum); 
    }


    #[instrument(level = "trace", skip(self, item_prices))]
    /// Update existing item price list with new entries
    /// Calls HashMap::extend
    pub fn update_item_prices(&mut self, item_prices: PriceDataType) {
        // TODO: Impl Iterator or some trait so don't have to call data field
        // self.items.extend(item_prices.data)

        for iprice in item_prices.data {
            let id = iprice.0;
            let name = match self.name_from_id(&id) {
                Some(n) => n.to_owned(),
                None => continue,
            };

            let price_data = iprice.1;

            let item = Item::new(name.clone(), id, price_data);
            self.items.insert(name, item);
        }

        self.add_special_values();
    }


    fn item_from_id_price(&self, name: String, price: PriceDatum) -> Option<Item>{
        let id = self.id_from_name(&name)?;

        Some(Item::new(name, id.clone(), price))
    }
}
