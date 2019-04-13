# Changelog

## 0.4.2

- Write unit tests
- Update crate description

## 0.4.1

- Update dependencies
- Implement `Eq` and `Ord` for `Grade`

## 0.4.0

- Updated dependencies
- `level_by_time` like interface is supported back again.
- `time` has the measured time of the station.

## 0.3.1

- implement `fmt::Display` trait for `Grade`

## 0.3.0

- Now with 2018 edition!
- Updated dependencies
- Asynchronous call with futures

## 0.2.0

- Now work properly with new airkorea mobile page
- `level_by_time` is no longer supported. Now only shows current status.
- Each `Pollutant` fields of `AirStatus` is now merged into `pollutants` vector.
