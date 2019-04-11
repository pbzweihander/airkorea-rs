//! # airkorea
//!
//! Airkorea API wrapper using Airkorea mobile page.
//!
//! # Example
//!
//! ```
//! # use tokio::runtime::Runtime;
//! # use futures::prelude::*;
//! # let mut rt = Runtime::new().unwrap();
//! # let (lng, lat) = (127.28698636603603, 36.61095403123917);
//! let status = rt.block_on(airkorea::search(lng, lat)).unwrap();
//! println!("Station address: {}", status.station_address);
//! println!("Time: {}", status.time);
//! for pollutant in status {
//!     println!("{}", pollutant);
//! }
//! ```

use {
    failure::Error,
    futures::prelude::*,
    lazy_static::lazy_static,
    regex::Regex,
    reqwest::r#async::Client,
    scraper::{Html, Selector},
    std::fmt,
};

#[derive(Clone, Debug)]
pub struct AirStatus {
    pub station_address: String,
    pub time: String,
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
    pub data: Vec<Option<f32>>,
    pub grade: Grade,
}

impl fmt::Display for Pollutant {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{:<6}({}): {}  {}",
            self.name,
            self.unit,
            self.data
                .iter()
                .map(|p| p.map(|f| f.to_string()).unwrap_or_else(|| "--".to_string()))
                .collect::<Vec<_>>()
                .join(" → "),
            self.grade,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
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
            Grade::Critical
        } else {
            Grade::None
        }
    }
}

impl fmt::Display for Grade {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Grade::None => "None",
                Grade::Good => "Good",
                Grade::Normal => "Normal",
                Grade::Bad => "Bad",
                Grade::Critical => "Critical",
            }
        )
    }
}

fn extract_text_from_element(element: scraper::element_ref::ElementRef) -> String {
    element
        .text()
        .map(|s| s.trim())
        .collect::<Vec<_>>()
        .join("")
}

fn request(url: &str) -> impl Future<Item = Html, Error = Error> {
    let client = Client::new();
    client
        .get(url)
        .send()
        .map_err(Into::into)
        .and_then(|resp| {
            resp.into_body().concat2().map_err(Into::into).map(|chunk| {
                let v = chunk.to_vec();
                String::from_utf8_lossy(&v).to_string()
            })
        })
        .map(|body| Html::parse_document(&body))
}

fn parse(document: &Html) -> AirStatus {
    lazy_static! {
        static ref SELECTOR_STATION: Selector = Selector::parse("h1>.tit").unwrap();
        static ref SELECTOR_TIME: Selector = Selector::parse("h1>.tim").unwrap();
        static ref SELECTOR_LIST: Selector = Selector::parse("div[class^=mList]>ul>li").unwrap();
        static ref SELECTOR_NAME: Selector = Selector::parse(".tit").unwrap();
        static ref REGEX_NAME: Regex = Regex::new(r"\((.*)\)").unwrap();
        static ref SELECTOR_GRADE: Selector = Selector::parse(".con>.co>.tx>.t1").unwrap();
        static ref SELECTOR_UNIT: Selector = Selector::parse(".con>.co>.tx>.t1>sub").unwrap();
        static ref SELECTOR_SCRIPT: Selector = Selector::parse("body>script:last-child").unwrap();
        static ref REGEX_ROW: Regex = Regex::new(r"addRows\(\[(.*)\]\);").unwrap();
    }

    let station_address = document
        .select(&SELECTOR_STATION)
        .map(|e| e.text().next().unwrap_or_default().trim().to_string())
        .next()
        .unwrap_or_default();
    let time = document
        .select(&SELECTOR_TIME)
        .map(extract_text_from_element)
        .next()
        .unwrap_or_default();

    let pollutant_keys = document.select(&SELECTOR_LIST).map(|graph| {
        let name = graph
            .select(&SELECTOR_NAME)
            .next()
            .map(extract_text_from_element)
            .and_then(|n| {
                REGEX_NAME
                    .captures(&n)
                    .and_then(|c| c.get(1))
                    .map(|c| c.as_str().to_string())
            });
        let grade = graph
            .select(&SELECTOR_GRADE)
            .next()
            .map(extract_text_from_element)
            .map(|g| Grade::from_str(&g))
            .unwrap_or_else(|| Grade::None);
        let unit = graph
            .select(&SELECTOR_UNIT)
            .next()
            .map(extract_text_from_element)
            .unwrap_or_default();

        (name, unit, grade)
    });

    let pollutants = document
        .select(&SELECTOR_SCRIPT)
        .next()
        .map(extract_text_from_element)
        .map(|script| {
            REGEX_ROW
                .find_iter(&script)
                .map(|row| {
                    let row = row.as_str();
                    row.split("],[")
                        .map(|data| data.split(',').filter_map(|s| s.parse::<f32>().ok()).next())
                        .collect::<Vec<_>>()
                })
                .zip(pollutant_keys)
                .filter_map(|(data, (name, unit, grade))| {
                    name.map(|name| Pollutant {
                        name,
                        unit,
                        data,
                        grade,
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    AirStatus {
        station_address,
        time,
        pollutants,
    }
}

pub fn search(longitude: f32, latitude: f32) -> impl Future<Item = AirStatus, Error = Error> {
    let addr = format!(
        "http://m.airkorea.or.kr/main?lng={}&lat={}&deviceID=1234",
        longitude, latitude
    );
    request(&addr).map(|html| parse(&html))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unreadable_literal, clippy::excessive_precision)]

    use {crate::*, tokio::runtime::Runtime};

    #[test]
    fn test() {
        let mut rt = Runtime::new().unwrap();
        let status = rt
            .block_on(search(127.28698636603603, 36.61095403123917))
            .unwrap();

        println!("{}", status.station_address);
        println!("{}", status.time);
        for pollutant in status {
            println!("{}", pollutant);
        }
    }
}
