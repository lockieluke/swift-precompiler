use serde::{Deserialize, Serialize};
use toml::Table;
use toml::value::Array;

#[derive(Serialize, Deserialize)]
pub(crate) struct Config {
    pub(crate) path_aliases: Option<Table>,
    pub(crate) dirs: Option<Array>
}

impl Config {

    pub(crate) fn default() -> Self {
        Self {
            path_aliases: Some(Table::new()),
            dirs: Some(Array::new())
        }
    }

}