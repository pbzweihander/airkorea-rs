# airkorea-rs

[![circleci](https://circleci.com/gh/pbzweihander/airkorea-rs.svg?style=shield)](https://circleci.com/gh/pbzweihander/airkorea-rs)
[![crate.io](https://img.shields.io/crates/v/airkorea.svg)](https://crates.io/crates/airkorea)
[![docs.rs](https://docs.rs/airkorea/badge.svg)](https://docs.rs/airkorea)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE-MIT)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE-APACHE)

Limitless [Airkorea](http://www.airkorea.or.kr) API wrapper written in Rust.

```rust
use {airkorea, futures::prelude::*, tokio::runtime::Runtime};

let mut rt = Runtime::new();
let status = rt.block_on(airkorea::search(lng, lat))?;
println!("Station address: {}", status.station_address);
for pollutant in status {
    println!("{}", pollutant);
}
```
