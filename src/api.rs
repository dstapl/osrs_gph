use std::collections::HashMap;

use serde::Deserialize;

use crate::config::Config;

#[derive(Debug, Deserialize)]
pub struct MappingItem {
    #[serde(default)]
    pub highalch: i32,
    pub members: bool,
    pub name: String,
    pub examine: String,
    pub id: i32,
    pub value: i32,
    pub icon: String,
    #[serde(default)]
    pub lowalch: i32,
}

pub struct Api {
    pub config: &crate::config::Api
    headers: ApiHeaders
}

pub struct ApiHeaders {
    pub headers: HashMap<String, String>
}


impl<S: Into<String>> From<HashMap<S,S>> for ApiHeaders {
    fn from(value: HashMap<S,S>) -> Self {
        let headers: HashMap<String, String> = value.into_iter()
            .map(|(x,y)| (x.into(), y.into()))
            .collect();
        ApiHeaders { headers } 
    }
}

impl Api {
    fn new<H: Into<ApiHeaders>>(config: &crate::config::Api, headers: H) -> Self {
        Api { config, headers: headers.into() }
    }
    
    fn add_headers<S: Into<String>>(self, headers: HashMap<S,S>) -> Self {
        
    }

    fn set_headers(self, headers: ApiHeaders) -> Result<(), Some(())> {
        self.headers = (HashMap::new()).into();

    }

    fn request() { todo!() }
}

fn request_item_prices(config: &Config) -> Result<(), std::io::Error> {
    let api: Api = Api::new(config.api);
    
}
