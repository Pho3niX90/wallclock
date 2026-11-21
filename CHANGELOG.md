# Changelog

All notable changes to this crate are listed here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.1.0

First release.

### Added

- `DateTime`: a wall-clock reading in a time zone with PEP 495's `fold`, matching Python's aware `datetime`.
  - Built from a date and time of day, from a timestamp in microseconds, or from the system clock.
  - Arithmetic on the wall clock (`checked_add`, `checked_sub`), difference (`since`), and Python's comparison rules (`py_cmp`, `py_eq`).
  - Conversion between zones (`astimezone`), with `fold` set on the second of two readings.
  - `utcoffset`, `tzname`, `timestamp`, `timestamp_micros`, `weekday`, `isoformat` and `Display`.
- `Tz`: an IANA zone (`zoneinfo.ZoneInfo`) or a fixed offset (`datetime.timezone`).
- `TimeDelta`: Python's `timedelta`, with its range, normalisation, `total_seconds` and text.
- `Error`, with the name of the exception Python raises in each case.
- The `bundled` feature, which compiles the time zone database into the program on every platform.
- Tests against answers recorded from CPython, and a sweep of every zone in the database.
