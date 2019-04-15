//! # airkorea
//!
//! Airkorea Crawler using Airkorea mobile page.
//!
//! ## Example
//!
//! ```no_run
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
//!
//! ## Testing
//!
//! You can override Airkorea Url for mock testing.
//! If you want to write unit tests for some functions using airkorea,
//! just set `AIRKOREA_URL` environment variable to desired mock server.
//!
//! ```no_run
//! # fn spawn_server(_: &str) {}
//! # let mut rt = tokio::runtime::Runtime::new().unwrap();
//! spawn_server("localhost:1234");
//! std::env::set_var("AIRKOREA_URL", "http://localhost:1234");
//! let status = rt.block_on(airkorea::search(123.123, 456.456)).unwrap();
//! assert_eq!(&status.station_address, "Foobar Station");
//! ```

use {
    failure::Error,
    futures::prelude::*,
    lazy_static::lazy_static,
    regex::Regex,
    reqwest::{r#async::Client, Url},
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
    element.text().map(str::trim).collect::<Vec<_>>().join("")
}

fn request(url: Url) -> impl Future<Item = Html, Error = Error> {
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
    static AIRKOREA_URL: &'static str = "http://m.airkorea.or.kr/main?deviceID=1234";

    let airkorea_url = std::env::var("AIRKOREA_URL").unwrap_or_else(|_| AIRKOREA_URL.to_string());

    let addr = Url::parse_with_params(
        &airkorea_url,
        &[
            ("lng", longitude.to_string()),
            ("lat", latitude.to_string()),
        ],
    )
    .unwrap_or_else(|why| {
        panic!(
            "Cannot parse url {}&lng={}&lat={}: {}",
            AIRKOREA_URL, longitude, latitude, why
        )
    });
    request(addr).map(|html| parse(&html))
}

#[cfg(test)]
mod tests {
    use {crate::*, hyper::Server, tokio::runtime::Runtime};

    #[test]
    fn test_request() {
        static HTML: &'static str = r#"<html>
<head><title>FooBar</title></head>
<body>Hello, world!</body>
</html>"#;

        let (called_sender, called_receiver) = std::sync::mpsc::channel();
        let (shutdown_sender, shutdown_receiver) = futures::sync::oneshot::channel();

        let mut rt = Runtime::new().unwrap();

        let service = hyper::service::make_service_fn(move |_| {
            let called_sender = called_sender.clone();
            hyper::service::service_fn_ok(move |_| {
                called_sender.send(()).unwrap();
                hyper::Response::new(hyper::Body::from(HTML))
            })
        });

        let server = Server::bind(&"0.0.0.0:12121".parse().unwrap())
            .serve(service)
            .with_graceful_shutdown(shutdown_receiver)
            .map_err(|why| panic!("{}", why));

        rt.spawn(server);

        let url = "http://localhost:12121".parse().unwrap();
        let fut = request(url)
            .map(|resp| {
                assert_eq!(resp, Html::parse_document(HTML));
            })
            .and_then(|_| shutdown_sender.send(()).map_err(|_| panic!("Cannot send")))
            .map_err(|why| panic!("{}", why));

        rt.block_on_all(fut).unwrap();

        called_receiver.try_recv().unwrap();
    }

    #[test]
    fn test_parse() {
        static HTML: &'static str = include_str!("../tests/test.html");

        let html = Html::parse_document(HTML);

        let status = parse(&html);

        assert_eq!(
            &status.station_address,
            "세종 세종시 신흥동측정소"
        );
        assert_eq!(&status.time, "2019-04-13 18시 기준");

        assert_eq!(&status.pollutants[0].name, "CAI");
        assert_eq!(&status.pollutants[0].unit, "");
        assert_eq!(status.pollutants[0].grade, Grade::Normal);
        assert_eq!(
            status.pollutants[0].data,
            vec![
                Some(74.0),
                Some(68.0),
                Some(63.0),
                Some(64.0),
                Some(65.0),
                Some(60.0),
                Some(63.0),
                Some(66.0),
                Some(74.0),
                Some(79.0),
                Some(79.0),
                Some(82.0),
                Some(79.0),
                Some(85.0),
                Some(92.0),
                Some(97.0),
                Some(100.0),
                Some(97.0),
                Some(90.0),
                Some(83.0),
                Some(83.0),
                Some(84.0),
                Some(85.0),
                Some(81.0),
            ]
        );

        assert_eq!(&status.pollutants[6].name, "SO2");
        assert_eq!(&status.pollutants[6].unit, "ppm");
        assert_eq!(status.pollutants[6].grade, Grade::Good);
        assert_eq!(
            status.pollutants[6].data,
            vec![
                Some(0.004),
                Some(0.003),
                Some(0.003),
                Some(0.003),
                Some(0.003),
                Some(0.003),
                Some(0.003),
                Some(0.003),
                Some(0.003),
                Some(0.003),
                Some(0.003),
                Some(0.002),
                Some(0.003),
                Some(0.003),
                Some(0.005),
                Some(0.005),
                Some(0.004),
                Some(0.004),
                Some(0.003),
                Some(0.003),
                Some(0.003),
                Some(0.003),
                Some(0.003),
                Some(0.003),
            ]
        );
    }

    #[test]
    fn test_extract_text_from_element() {
        static HTML: &'static str = "<p>foo<span>bar<h1>baz</h1></span></p>";

        let html = Html::parse_fragment(HTML);
        let element = html.root_element();
        let text = extract_text_from_element(element);

        assert_eq!(&text, "foobarbaz");
    }
}
