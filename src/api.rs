use std::{
    collections::HashMap,
    io::{BufReader, Read},
};

use reqwest::{blocking, header::HeaderMap};
use serde::Deserialize;

use crate::{item_search::data_types, log_match_panic, log_panic};

use tracing::{instrument, trace, warn};

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

#[derive(Debug, Clone, Copy)]
// [Latest](Timespan::Latest) will return the latest high and low prices
// [Oldest](Timespan::Oldest) will provide an average of the prices for:
//  `5` minutes
//  `1` hour
pub enum Timespan {
    Latest,
    Oldest(u16), // 5(minutes), 1(hour)
                 // TODO: Implement *SPECIFIC ITEM* timeseries? Not really needed...
                 // Maybe only if the item can't be found in the current timespan?
                 // -> Increase the lookback time
                 //Series(u16), // 5(minutes), 1(hours), 6(hours), 24(hours)
}

#[derive(Debug)]
pub struct Api {
    url: String,
    timespan: Timespan,
    headers: ApiHeaders,
}

#[derive(Debug)]
pub struct ApiHeaders {
    pub headers: HashMap<String, String>,
}

impl From<crate::config::TimeSpan> for Timespan {
    fn from(span: crate::config::TimeSpan) -> Self {
        match span {
            crate::config::TimeSpan::Latest => Self::Latest,
            crate::config::TimeSpan::FiveMinute => Self::Oldest(5),
            crate::config::TimeSpan::OneHour => Self::Oldest(1),
        }
    }
}

impl Timespan {
    // TODO: Include `/` in endpoint String?
    fn get_endpoint(self) -> String {
        match self {
            Self::Latest => "/latest",
            Self::Oldest(t) => match t {
                5 => "/5m",
                1 => "/1h",
                unknown => log_panic("Unimplemented timespan", unknown),
            },
        }
        .to_string()
    }
}

impl<S: Into<String>> From<HashMap<S, S>> for ApiHeaders {
    fn from(value: HashMap<S, S>) -> Self {
        let headers: HashMap<String, String> = value
            .into_iter()
            .map(|(x, y)| (x.into(), y.into()))
            .collect();
        ApiHeaders { headers }
    }
}

impl<K, V> Extend<(K, V)> for ApiHeaders
where
    K: Into<String>,
    V: Into<String>,
{
    #[inline]
    fn extend<T: IntoIterator<Item = (K, V)>>(&mut self, iter: T) {
        self.headers
            .extend(iter.into_iter().map(|(k, v)| (k.into(), v.into())));
    }
}

impl Api {
    pub fn new(api_config: &crate::config::Api) -> Self {
        //Api { config: config, headers: config.auth_headers.clone().into()}
        Api {
            url: api_config.url.clone(),
            timespan: Timespan::from(api_config.timespan.clone()),
            headers: ApiHeaders::from(api_config.auth_headers.clone()),
        }
    }

    /// TODO: Make argument a String instead?
    #[instrument(level = "trace", skip(self))]
    pub fn set_timespan(&mut self, timespan: Timespan) {
        // TODO: Can you remove this trace! ?
        // Is `instrument` logging as well?
        trace!(desc = "Setting timespan", timespan = ?timespan);
        self.timespan = timespan
    }

    /// Updates existing API [headers](Api::headers) with provided `headers`
    #[instrument(level = "trace", skip(self, headers))]
    pub fn add_headers<S: Into<String>>(&mut self, headers: HashMap<S, S>) {
        trace!(desc = "Extending headers");
        trace!(old = ?self.headers);

        self.headers.extend(headers);

        trace!(new = ?self.headers);
    }

    /// Replaces existing API [headers](Api::headers) with provided `headers`
    #[instrument(level = "trace", skip(self, headers))]
    pub fn set_headers(&mut self, headers: ApiHeaders) {
        trace!(desc = "Overwriting headers");
        trace!(old = ?self.headers);

        self.headers = headers;

        trace!(new = ?self.headers);
    }

    /// Make a request to the [config url](Api::config::api::url)
    /// At the current endpoint
    #[tracing::instrument(name = "api::request")]
    pub fn request_item_prices(&self) -> data_types::latest::PriceDataType {
        // TODO: Optimise by storing headers as HeaderMap in API struct?
        let header_map: HeaderMap = log_match_panic(
            HeaderMap::try_from(&self.headers.headers),
            "Made HeaderMap",
            "HeaderMap conversion error",
        );

        let endpoint: String = self.timespan.get_endpoint();
        let target: String = self.url.clone() + &endpoint;

        let client = blocking::Client::new();
        let res_build = client.get(target).headers(header_map);

        let mut res = log_match_panic(res_build.send(), "Recieved response", "Request sent error");

        // Decode response
        let buffer = BufReader::new(res.by_ref());

        log_match_panic(
            serde_yaml_ng::from_reader(buffer),
            "Deserializing API response",
            "Failed to deserialize API response",
        )
    }

    /// Wrapper around [`self.request_item_prices`]
    pub fn request_timespan_prices(
        &mut self,
        timespan: Timespan,
    ) -> data_types::latest::PriceDataType {
        let old_timespan = self.timespan;

        self.timespan = timespan;
        let res = self.request_item_prices();

        // Restore actual timespan
        self.timespan = old_timespan;

        res
    }
}
