extern crate airkorea;

#[test]
fn test() {
    let status = airkorea::search(127.28698636603603, 36.61095403123917).unwrap();
    assert_eq!(
        status.station_address,
        "세종 세종시 신흥동측정소".to_string()
    );
    for pollutant in status {
        println!("{}", pollutant);
    }
}
