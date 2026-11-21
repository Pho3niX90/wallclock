# wallclock

[![CI](https://github.com/Pho3niX90/wallclock/actions/workflows/ci.yml/badge.svg)](https://github.com/Pho3niX90/wallclock/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/wallclock.svg)](https://crates.io/crates/wallclock)
[![docs.rs](https://docs.rs/wallclock/badge.svg)](https://docs.rs/wallclock)

Datetimes in time zones for Rust, with arithmetic on the wall clock. It behaves as Python's `datetime` and `zoneinfo` do, and every rule is checked against CPython.

## Why

"Add one day" to a time in a time zone can mean two things. Python moves the wall clock: 08:00 in Madrid plus one day is 08:00 in Madrid the next day, including on the night the clocks change, when that day is 23 or 25 hours long. A Rust program that schedules things for people ("every day at 08:00"), or that has to agree with a Python program, needs that answer. wallclock gives it.

```rust
use wallclock::{DateTime, TimeDelta, Tz};

fn main() -> Result<(), wallclock::Error> {
    let madrid = Tz::zone("Europe/Madrid")?;
    // 08:00 on the day before the clocks go back.
    let at = DateTime::new(2026, 10, 24, &madrid)?.with_time(8, 0, 0, 0)?;
    let next = at.checked_add(TimeDelta::DAY)?;

    // Still 08:00 on the wall clock...
    assert_eq!(next.isoformat(), "2026-10-25T08:00:00+01:00");
    // ...which is 25 hours later.
    assert_eq!(next.timestamp_micros() - at.timestamp_micros(), 25 * 3_600_000_000);
    Ok(())
}
```

The same in Python:

```python
at = datetime(2026, 10, 24, 8, tzinfo=ZoneInfo("Europe/Madrid"))
(at + timedelta(days=1)).isoformat()   # '2026-10-25T08:00:00+01:00'
```

## Install

```toml
[dependencies]
wallclock = "0.1"
```

Rust 1.85 or newer. On a system with no time zone database of its own (a minimal container, for example), turn on the `bundled` feature; see [the guide](docs/guide.md#the-time-zone-database).

## What is in it

| Type | Python's | What it is |
|---|---|---|
| `DateTime` | `datetime` with a `tzinfo` | A wall-clock reading in a zone, with PEP 495's `fold` |
| `Tz` | `zoneinfo.ZoneInfo`, `timezone` | An IANA zone such as `Europe/Madrid`, or a fixed offset |
| `TimeDelta` | `timedelta` | A length of time, to the microsecond |
| `Error` | `OverflowError`, `ValueError`, `ZoneInfoNotFoundError` | What went wrong, named as Python names it |

It follows Python where Python is particular:

- **Repeated and skipped hours.** When the clocks go back an hour happens twice, and `fold` picks the second reading. When they go forward an hour is skipped; a time in it is read by the rules before the change, or with `fold` by those after.
- **Same zone or not.** Two datetimes in one zone are compared and subtracted by their wall clocks. Datetimes in different zones go by their instants.
- **Range.** Years 1 to 9999. Going outside is an error, never a clamped value.
- **Daylight saving in the far future.** The rule each zone ends with keeps applying, in the year 9000 as now.

It leaves out naive datetimes, parsing (`fromisoformat`, `strptime`), `strftime`, `dst()`, and `date` and `time` on their own.

## Documentation

- **[Guide](docs/guide.md)**: how wallclock thinks about time, clock changes, comparison, zones, errors, and recipes for common jobs.
- **[Coming from Python](docs/from-python.md)**: a line-by-line translation table and the places where Rust makes it look different.
- **[API reference](https://docs.rs/wallclock)** on docs.rs, with an example on every function.
- **[Contributing](CONTRIBUTING.md)**: how the tests against CPython work and how to add to them.
- **[Changelog](CHANGELOG.md)**.

## How it is tested

Everything is compared with CPython.

- **Recorded answers.** About 9,750 answers from CPython are kept in the repository and checked on every `cargo test`: offsets, zone names, timestamps and text around every kind of clock change, sums, conversions, comparisons and timedeltas.
- **The sweep.** CI asks CPython about every zone in the time zone database over its whole history, about 750,000 answers across some 480 zones, and expects this crate to give the same.
- **The documentation.** Every Rust example in this README, the guide and the API reference is compiled and run.

The details are in [CONTRIBUTING.md](CONTRIBUTING.md).

## Licence

MIT. See [LICENSE](LICENSE).
