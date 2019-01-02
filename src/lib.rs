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
                    .unwrap_or_else(|| "--".to_string()),
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
            Grade::Critical
        } else {
            Grade::None
        }
    }
}

fn extract_text_from_element(element: scraper::element_ref::ElementRef) -> String {
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

fn parse(document: &Html) -> Result<AirStatus, Error> {
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
        })
        .filter_map(|(name, level, grade)| {
            REGEX_UNWRAP.captures(&name).map(|c| {
                let name = c.get(1).unwrap().as_str().to_string();
                (name, level, grade)
            })
        })
        .filter_map(|(name, level, grade)| {
            REGEX_LEVEL.captures(&level).map(|c| {
                let level = c.get(1).unwrap().as_str().to_string();
                let unit = c.get(2).unwrap().as_str().to_string();
                (name, level, unit, grade)
            })
        })
        .map(|(name, level, unit, grade)| {
            let level = level.parse::<f32>().ok();
            let grade = Grade::from_str(&grade);

            Pollutant {
                name: name,
                level: level,
                unit: unit,
                grade: grade,
            }
        })
        .collect::<Vec<_>>();

    Ok(AirStatus {
        station_address,
        pollutants,
    })
}

pub fn search(longitude: f32, latitude: f32) -> impl Future<Item = AirStatus, Error = Error> {
    use futures::future::result;
    let addr = format!(
        "http://m.airkorea.or.kr/main?lng={}&lat={}&deviceID=1234",
        longitude, latitude
    );
    request(&addr).and_then(|html| result(parse(&html)))
}
