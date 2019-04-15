# airkorea-rs

[![circleci](https://circleci.com/gh/pbzweihander/airkorea-rs.svg?style=shield)](https://circleci.com/gh/pbzweihander/airkorea-rs)
[![crate.io](https://img.shields.io/crates/v/airkorea.svg)](https://crates.io/crates/airkorea)
[![docs.rs](https://docs.rs/airkorea/badge.svg)](https://docs.rs/airkorea)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE-MIT)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE-APACHE)

[Airkorea](http://www.airkorea.or.kr) Crawler written in Rust.

## Usage

```rust
use {airkorea, futures::prelude::*, tokio::runtime::Runtime};

let mut rt = Runtime::new();

let status = rt.block_on(airkorea::search(lng, lat))?;

println!("Station address: {}", status.station_address);
println!("Time: {}", status.time);
for pollutant in status {
    println!("{}", pollutant);
}
```

## Testing

You can override Airkorea Url for mock testing.
If you want to write unit tests for some functions using airkorea,
just set `AIRKOREA_URL` environment variable to desired mock server.

```rust
spawn_server("localhost:1234");

std::env::set_var("AIRKOREA_URL", "http://localhost:1234");

let status = rt.block_on(airkorea::search(123.123, 456.456)).unwrap();

assert_eq!(&status.station_address, "Foobar Station");
```
