use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct MappingItem {
    #[serde(default)]
    highalch: i32,
    members: bool,
    name: String,
    examine: String,
    id: i32,
    value: i32,
    icon: String,
    #[serde(default)]
    lowalch: i32,
}
