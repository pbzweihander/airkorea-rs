#[macro_use]
extern crate error_chain;
extern crate serde;
extern crate serde_hjson;
extern crate serde_json;

pub mod errors;
pub use errors::{Error, Result};

pub fn convert(s: &str) -> Result<String> {
    let value: serde_hjson::Value = serde_hjson::from_str(s)?;
    serde_json::to_string_pretty(&value).map_err(|e| e.into())
}
