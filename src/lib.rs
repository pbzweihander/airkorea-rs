//! # airkorea
//!
//! Airkorea API wrapper using Airkorea mobile page.
//!
//! # Example
//!
//! ```ignore
//! let status = airkorea::search(lng, lat)?;
//! println!("Station address: {}", status.station_address);
//! for pollutant in status {
//!     println!("{}", pollutant);
//! }
//! ```

#[macro_use]
extern crate error_chain;
extern crate hjson2json;
extern crate kuchiki;
#[macro_use]
extern crate lazy_static;
extern crate regex;
extern crate reqwest;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

pub mod errors;
pub use errors::{Error, ErrorKind, Result};

use kuchiki::traits::TendrilSink;
use std::collections::HashMap;
use std::fmt;
use regex::{Regex, RegexBuilder};

#[derive(Deserialize)]
struct Object {
    cols: Vec<Col>,
    rows: Vec<Row>,
}

#[derive(Deserialize)]
struct Row {
    c: Vec<Item>,
}

#[derive(Deserialize)]
struct Col {
    label: String,
}

#[derive(Deserialize, Clone)]
struct Item {
    v: Value,
    f: Option<String>,
}

#[derive(Deserialize, Clone)]
#[serde(untagged)]
enum Value {
    String(String),
    Float(f32),
    None,
}

impl Value {
    fn to_float(&self) -> Option<f32> {
        match *self {
            Value::Float(f) => Some(f),
            _ => None,
        }
    }
}

#[derive(Clone)]
pub struct AirStatus {
    pub station_address: String,
    pub pm10: Pollutant,
    pub pm25: Pollutant,
    pub o3: Pollutant,
    pub no2: Pollutant,
    pub co: Pollutant,
    pub so2: Pollutant,
}

impl IntoIterator for AirStatus {
    type Item = Pollutant;
    type IntoIter = std::vec::IntoIter<Pollutant>;

    fn into_iter(self) -> Self::IntoIter {
        vec![self.pm10, self.pm25, self.o3, self.no2, self.co, self.so2].into_iter()
    }
}

impl AirStatus {
    pub fn into_map(self) -> HashMap<String, Pollutant> {
        vec![
            ("pm10", self.pm10),
            ("pm25", self.pm25),
            ("o3", self.o3),
            ("no2", self.no2),
            ("co", self.co),
            ("so2", self.so2),
        ].into_iter()
            .map(|(key, pollutant)| (key.to_owned(), pollutant))
            .collect()
    }
}

#[derive(Clone)]
pub struct Pollutant {
    pub name: String,
    pub unit: String,
    pub level_by_time: Vec<Option<f32>>,
    pub grade: Grade,
}

impl fmt::Display for Pollutant {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} ({}): {} {}",
            self.name,
            self.unit,
            (&self.level_by_time)
                .into_iter()
                .map(|l| match *l {
                    Some(f) => f.to_string(),
                    None => String::new(),
                })
                .collect::<Vec<String>>()
                .join(" -> "),
            match self.grade {
                Grade::None => "",
                Grade::Good => "(Good)",
                Grade::Normal => "(Normal)",
                Grade::Bad => "(Bad)",
                Grade::Critical => "(Critical)",
            }
        )
    }
}

#[derive(Clone)]
pub enum Grade {
    None,
    Good,
    Normal,
    Bad,
    Critical,
}

pub fn search(longitude: f32, latitude: f32) -> Result<AirStatus> {
    let addr = format!(
        "http://m.airkorea.or.kr/sub_new/sub11.jsp?lng={}&lat={}",
        longitude, latitude
    );
    let mut resp = reqwest::get(&addr)?;
    let document = kuchiki::parse_html().one(resp.text()?);

    let script = document
        .select_first("head script[type=\"text/javascript\"]")
        .map_err(|_| ErrorKind::ParsePage)?
        .text_contents();
    let station_address = document
        .select_first("#doc_addr")
        .map_err(|_| ErrorKind::ParsePage)?
        .text_contents();

    lazy_static! {
        static ref REGEX_GRADE: Regex =
            RegexBuilder::new("^(?:\\s+)function view([^(]+)[\\W\\n]+var grade = +\"(\\d)\";$")
                .dot_matches_new_line(true)
                .multi_line(true)
                .build()
                .unwrap();
        static ref REGEX_HJSON: Regex =
            RegexBuilder::new(r"^var JSONObject([^_]+)_3 = (.+?);")
                .dot_matches_new_line(true)
                .multi_line(true)
                .build()
                .unwrap();
    }
    let grades = REGEX_GRADE
        .captures_iter(&script)
        .map(|capture| {
            let id = capture.get(1).map(|m| m.as_str().to_lowercase());
            let grade = capture
                .get(2)
                .and_then(|m| m.as_str().parse::<usize>().ok())
                .map(|grade| match grade {
                    1 => Grade::Good,
                    2 => Grade::Normal,
                    3 => Grade::Bad,
                    4 => Grade::Critical,
                    _ => Grade::None,
                })
                .unwrap_or_else(|| Grade::None);
            (id, grade)
        })
        .filter_map(|id_and_grade| match id_and_grade {
            (Some(id), grade) => Some((id, grade)),
            _ => None,
        })
        .collect::<HashMap<String, Grade>>();

    let mut lists = REGEX_HJSON
        .captures_iter(&script)
        .map(|capture| {
            let id = capture.get(1).map(|m| m.as_str().to_owned());
            let name_and_levels = capture.get(2).and_then(|m| to_object(m.as_str()).ok()).map(
                |o| {
                    let name = o.cols.into_iter().nth(1).map(|c| c.label).unwrap();
                    let level_by_time = o.rows.into_iter().flat_map(|r| {
                        r.c.into_iter().nth(1).map(|item| {
                            let level = item.v.to_float();
                            let unit = item.f.and_then(|unit| to_unit(&unit).ok());
                            (level, unit)
                        })
                    });
                    (name, level_by_time)
                },
            );
            (id, name_and_levels)
        })
        .filter_map(|element| match element {
            (Some(id), Some((name, level_by_time))) => Some((id, name, level_by_time)),
            _ => None,
        })
        .map(|(id, name, mut level_by_time)| {
            let unit = level_by_time
                .by_ref()
                .filter_map(|(_, unit)| unit)
                .next()
                .unwrap_or_else(|| "".to_owned());
            let pollutant = Pollutant {
                name,
                unit,
                level_by_time: level_by_time.map(|(level, _)| level).collect(),
                grade: grades.get(&id).cloned().unwrap_or_else(|| Grade::None),
            };
            (id, pollutant)
        })
        .collect::<HashMap<String, Pollutant>>();

    Ok(AirStatus {
        station_address,
        pm10: lists.remove("pm10").unwrap(),
        pm25: lists.remove("pm25").unwrap(),
        o3: lists.remove("o3").unwrap(),
        no2: lists.remove("no2").unwrap(),
        co: lists.remove("co").unwrap(),
        so2: lists.remove("so2").unwrap(),
    })
}

fn trim(s: &str) -> String {
    lazy_static! {
        static ref REGEX_WHITESPACE: Regex =
            Regex::new(r"[\s\t\r\n]")
                .unwrap();
    }
    REGEX_WHITESPACE.replace_all(s, "").into_owned()
}

fn padding(s: &str) -> String {
    lazy_static!{
        static ref REGEX_BRACKET: Regex =
            Regex::new(r"([\{\[,])|([\}\]])")
                .unwrap();
    }
    REGEX_BRACKET.replace_all(s, "$1\n$2").into_owned()
}

fn to_object(s: &str) -> Result<Object> {
    Ok(s)
        .map(trim)
        .map(|ref s| padding(s))
        .and_then(|hjson| {
            hjson2json::convert(&hjson.replace("'", "\"").replace(".,", ".0,"))
                .map_err(|e| e.into())
        })
        .and_then(|json| serde_json::from_str(&json).map_err(|e| e.into()))
}

fn to_unit(s: &str) -> Result<String> {
    lazy_static! {
        static ref REGEX_UNIT: Regex =
            Regex::new(r"[^.\d]+$").unwrap();
    }
    REGEX_UNIT
        .find(s)
        .map(|unit| unit.as_str().to_owned())
        .ok_or_else(|| ErrorKind::ParsePage.into())
}
