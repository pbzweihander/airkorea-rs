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

extern crate failure;
extern crate scraper;
#[macro_use]
extern crate lazy_static;
extern crate regex;
extern crate reqwest;

use regex::Regex;
use scraper::{Html, Selector};
use std::fmt;

pub use failure::Error;
pub type Result<T> = std::result::Result<T, failure::Error>;

#[derive(Clone, Debug)]
pub struct AirStatus {
    pub station_address: String,
    pub pollutants: Vec<Pollutant>,
}

impl IntoIterator for AirStatus {
    type Item = Pollutant;
    type IntoIter = std::vec::IntoIter<Pollutant>;

    fn into_iter(self) -> Self::IntoIter {
        self.pollutants.into_iter()
    }
}

#[derive(Clone, Debug)]
pub struct Pollutant {
    pub name: String,
    pub unit: String,
    pub level: Option<f32>,
    pub grade: Grade,
}

impl fmt::Display for Pollutant {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{:<6} {:<10} {}",
            self.name,
            format!(
                "{}{}",
                self.level
                    .map(|f| f.to_string())
                    .unwrap_or("--".to_string()),
                self.unit
            ),
            match self.grade {
                Grade::None => "None",
                Grade::Good => "Good",
                Grade::Normal => "Normal",
                Grade::Bad => "Bad",
                Grade::Critical => "Critical",
            }
        )
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Grade {
    None,
    Good,
    Normal,
    Bad,
    Critical,
}

impl Grade {
    fn from_str(s: &str) -> Self {
        if s.starts_with("좋") {
            Grade::Good
        } else if s.starts_with("보") {
            Grade::Normal
        } else if s.starts_with("나") {
            Grade::Bad
        } else if s.starts_with("매") {
            Grade::Bad
        } else {
            Grade::None
        }
    }
}

fn extract_text_from_element<'a>(element: scraper::element_ref::ElementRef<'a>) -> String {
    element
        .text()
        .map(|s| s.trim())
        .collect::<Vec<_>>()
        .join("")
}

fn extract_text_with_selector<'a>(
    element: &scraper::element_ref::ElementRef<'a>,
    selector: &Selector,
) -> String {
    element
        .select(selector)
        .map(extract_text_from_element)
        .collect::<Vec<_>>()
        .join("")
}

fn request(url: &str) -> Result<Html> {
    let mut resp = reqwest::get(url)?;
    Ok(Html::parse_document(&resp.text()?))
}

fn parse(document: Html) -> Result<AirStatus> {
    lazy_static! {
        static ref SELECTOR_STATION: Selector = Selector::parse(".tit").unwrap();
        static ref SELECTOR_ITEM: Selector = Selector::parse(".item").unwrap();
        static ref SELECTOR_NAME: Selector = Selector::parse(".ti>.t1").unwrap();
        static ref SELECTOR_LEVEL: Selector = Selector::parse(".ti>.t2").unwrap();
        static ref SELECTOR_GRADE: Selector = Selector::parse(".tx>.t").unwrap();
        static ref REGEX_UNWRAP: Regex = Regex::new("\\((.+)\\)").unwrap();
        static ref REGEX_LEVEL: Regex = Regex::new("([\\d.-]+)(.+)").unwrap();
    }

    let station_address = document
        .select(&SELECTOR_STATION)
        .map(|e| e.text().next().unwrap_or_default().trim().to_string())
        .next()
        .unwrap_or_default();
    let pollutants = document
        .select(&SELECTOR_ITEM)
        .map(|item| {
            let name = extract_text_with_selector(&item, &SELECTOR_NAME);
            let level = extract_text_with_selector(&item, &SELECTOR_LEVEL);
            let grade = extract_text_with_selector(&item, &SELECTOR_GRADE);
            (name, level, grade)
        }).filter_map(|(name, level, grade)| {
            REGEX_UNWRAP.captures(&name).map(|c| {
                let name = c.get(1).unwrap().as_str().to_string();
                (name, level, grade)
            })
        }).filter_map(|(name, level, grade)| {
            REGEX_LEVEL.captures(&level).map(|c| {
                let level = c.get(1).unwrap().as_str().to_string();
                let unit = c.get(2).unwrap().as_str().to_string();
                (name, level, unit, grade)
            })
        }).map(|(name, level, unit, grade)| {
            let level = level.parse::<f32>().ok();
            let grade = Grade::from_str(&grade);

            Pollutant {
                name: name,
                level: level,
                unit: unit,
                grade: grade,
            }
        }).collect::<Vec<_>>();

    Ok(AirStatus {
        station_address,
        pollutants,
    })
}

pub fn search(longitude: f32, latitude: f32) -> Result<AirStatus> {
    let addr = format!(
        "http://m.airkorea.or.kr/main?lng={}&lat={}&deviceID=1234",
        longitude, latitude
    );
    let html = request(&addr)?;
    let status = parse(html)?;
    Ok(status)
}
