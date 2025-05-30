//! Parsing items from [api](src/api.rs) data
//! TODO: Support for different modules (timespans) other than just latest
use super::data_types::latest::{self, PriceDataType};//::PriceDatum;
use super::recipes; 

use tracing::{debug, info, error, span, trace, warn, Level};

use serde::{de::Visitor, Deserialize};
use std::{collections::HashMap, fmt::Debug, hash::Hash, path::Path};


use crate::file_io::FileOptions;
use crate::{file_io, log_match_err};
use crate::config::FilePaths;

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
        self.item_prices.invalid_data()
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



pub struct ItemSearch{
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
        Self {
            items, // Using Item Name(String)=>Item(Object)
            filepaths,
            api_config,
            name_to_id: HashMap::new(),
            id_to_name: HashMap::new(),
        }
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
}

impl ItemSearch {
    /// Either from file (ideally) or from the api
    pub fn get_item_prices(&mut self, from_file: bool) -> PriceDataType {
        let res = if from_file {
            log_match_err(self.find_prices_from_file(),
                "Attempting to find prices from a stored mapping file.",
                "Failed to find prices from file. May not exist or malformed data."
            )
        } else {
            let api = crate::api::Api::new(&self.api_config);
            api.request_item_prices()
        };

        res
    }


    /// Attempts to load item prices
    /// from a file defined in config
    fn find_prices_from_file(&mut self) -> Result<PriceDataType, std::io::Error>{
        // todo!()
        // Get correct file name and try to load contents
        let price_io = crate::file_io::FileIO::new(
           self.filepaths.price_data.clone(),
           FileOptions::new(true, false, false),
        );

       price_io.read_serialized(file_io::SerChoice::YAML)
    }
}
