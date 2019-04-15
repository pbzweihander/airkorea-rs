#![allow(clippy::unreadable_literal, clippy::excessive_precision)]

use {
    airkorea::*, futures::prelude::*, hyper::Server, lazy_static::lazy_static,
    tokio::runtime::Runtime,
};

lazy_static! {
    static ref ENV_AIRKOREA_URL_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());
}

#[test]
fn integration_test() {
    static HTML: &'static str = include_str!("test.html");
    static LNG: f32 = 123.12312;
    static LAT: f32 = 456.45645;

    let (called_sender, called_receiver) = std::sync::mpsc::channel();
    let (shutdown_sender, shutdown_receiver) = futures::sync::oneshot::channel();

    let mut rt = Runtime::new().unwrap();

    let service = hyper::service::make_service_fn(move |_| {
        let called_sender = called_sender.clone();

        hyper::service::service_fn_ok(move |req| {
            let url = req.uri();

            called_sender.send(()).unwrap();

            assert_eq!(url.query().unwrap(), &format!("lng={}&lat={}", LNG, LAT));

            hyper::Response::new(hyper::Body::from(HTML))
        })
    });

    let server = Server::bind(&"0.0.0.0:12121".parse().unwrap())
        .serve(service)
        .with_graceful_shutdown(shutdown_receiver)
        .map_err(|why| panic!("{}", why));

    rt.spawn(server);

    let env_lock = ENV_AIRKOREA_URL_MUTEX.lock().unwrap();

    std::env::set_var("AIRKOREA_URL", "http://localhost:12121");

    let fut = search(LNG, LAT)
        .map(|status| {
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
        })
        .and_then(|_| shutdown_sender.send(()).map_err(|_| panic!("Cannot send")))
        .map_err(|why| panic!("{}", why));

    rt.block_on(fut).unwrap();
    std::env::remove_var("AIRKOREA_URL");

    drop(env_lock);

    called_receiver.try_recv().unwrap();
}

#[test]
fn integration_test_to_real_server() {
    let mut rt = Runtime::new().unwrap();

    let (lng, lat) = (127.28698636603603, 36.61095403123917);

    let env_lock = ENV_AIRKOREA_URL_MUTEX.lock().unwrap();

    std::env::remove_var("AIRKOREA_URL");
    let status = rt.block_on(search(lng, lat)).unwrap();

    drop(env_lock);

    assert!(!status.station_address.is_empty());
    assert!(!status.time.is_empty());
    assert_eq!(status.pollutants.len(), 7);
    for p in status.pollutants {
        assert!(!p.name.is_empty());
        assert!(!p.unit.is_empty() || p.name == "CAI");
        assert!(!p.data.is_empty());
    }
}
