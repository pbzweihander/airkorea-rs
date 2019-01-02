#![allow(clippy::unreadable_literal, clippy::excessive_precision)]

use {airkorea, tokio::runtime::Runtime};

#[test]
fn test() {
    let mut rt = Runtime::new().unwrap();
    let status = rt
        .block_on(airkorea::search(127.28698636603603, 36.61095403123917))
        .unwrap();
    assert_eq!(
        status.station_address,
        "세종 세종시 신흥동측정소".to_string()
    );
    for pollutant in status {
        println!("{}", pollutant);
    }
}
