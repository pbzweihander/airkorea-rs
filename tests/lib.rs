extern crate airkorea;

#[test]
fn test() {
    let status = airkorea::search(127.28698636603603, 36.61095403123917).unwrap();
    assert_eq!(status.station_address, "세종 조치원읍 군청로 87-16(신흥동) 세종특별자치시 조치원청사 옥상".to_owned());
    for pollutant in status {
        println!("{}", pollutant);
    }
}
