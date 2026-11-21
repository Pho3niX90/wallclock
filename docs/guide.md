# Guide

This guide explains how wallclock thinks about time, and then shows the common jobs. Every Rust example here is compiled and run by `cargo test`.

- [Two ways to read a time](#two-ways-to-read-a-time)
- [Making a datetime](#making-a-datetime)
- [Adding and subtracting time](#adding-and-subtracting-time)
- [When the clocks change](#when-the-clocks-change)
- [Comparing two datetimes](#comparing-two-datetimes)
- [Moving between zones](#moving-between-zones)
- [Text](#text)
- [Errors and the range of years](#errors-and-the-range-of-years)
- [The time zone database](#the-time-zone-database)
- [Recipes](#recipes)

## Two ways to read a time

A moment can be written down in two ways.

- **The instant**: a point on the one timeline everybody shares. A Unix timestamp is an instant.
- **The wall clock**: what a clock on the wall shows in some place, such as "08:00 on 24 October in Madrid".

Most of the year the two move together. They part twice a year in places with daylight saving time. On the night of 24 to 25 October 2026 the clocks in Madrid go back from 03:00 to 02:00, so that day lasts 25 hours:

| Wall clock in Madrid | Instant (UTC) | Hours since the row above |
|---|---|---|
| 24 October, 08:00 (UTC+2) | 24 October, 06:00 | |
| 25 October, 08:00 (UTC+1) | 25 October, 07:00 | 25 |

So "one day after 08:00 on the 24th" has two honest answers: 08:00 on the 25th (one day on the wall clock) or 07:00 on the 25th (24 hours on the timeline).

Python picks the wall clock. `datetime + timedelta(days=1)` keeps the time of day and lets the instant land where it lands. wallclock does the same, which is the reason it exists: a Rust program that has to agree with Python code, or that schedules things for people ("every day at 08:00"), needs that answer.

A `DateTime` holds three things: the wall-clock reading, the zone, and a flag called `fold` that is explained [below](#when-the-clocks-change). The instant is worked out from them when you ask for it.

## Making a datetime

A zone is either a named zone from the IANA database or a fixed offset from UTC.

```rust
use wallclock::{DateTime, Tz};

fn main() -> Result<(), wallclock::Error> {
    let madrid = Tz::zone("Europe/Madrid")?;
    let india = Tz::fixed(5 * 3600 + 30 * 60)?; // five and a half hours ahead of UTC

    // A date is midnight on that date; with_time sets the time of day.
    let meeting = DateTime::new(2026, 10, 24, &madrid)?.with_time(8, 0, 0, 0)?;
    assert_eq!(meeting.isoformat(), "2026-10-24T08:00:00+02:00");

    // From an instant: microseconds since 1970-01-01T00:00:00 UTC.
    let launch = DateTime::from_timestamp_micros(1_700_000_000_000_000, &india)?;
    assert_eq!(launch.isoformat(), "2023-11-15T03:43:20+05:30");

    // The system clock, in a zone.
    let now = DateTime::now(&Tz::UTC);
    assert!(now.year() >= 2026);
    Ok(())
}
```

The parts are read back with `year()`, `month()`, `day()`, `hour()`, `minute()`, `second()`, `microsecond()` and `weekday()` (Monday is 0).

## Adding and subtracting time

`checked_add` and `checked_sub` move the wall clock by a `TimeDelta` and keep the zone.

```rust
use wallclock::{DateTime, TimeDelta, Tz};

fn main() -> Result<(), wallclock::Error> {
    let madrid = Tz::zone("Europe/Madrid")?;
    let at = DateTime::new(2026, 10, 24, &madrid)?.with_time(8, 0, 0, 0)?;

    let next_day = at.checked_add(TimeDelta::DAY)?;
    assert_eq!(next_day.isoformat(), "2026-10-25T08:00:00+01:00"); // still 08:00

    // The instants are 25 hours apart, because the clocks went back in between.
    let hours = (next_day.timestamp_micros() - at.timestamp_micros()) / 3_600_000_000;
    assert_eq!(hours, 25);

    // Any length of time: 90 minutes.
    let later = at.checked_add(TimeDelta::from_minutes(90)?)?;
    assert_eq!(later.isoformat(), "2026-10-24T09:30:00+02:00");
    Ok(())
}
```

This holds for hours as well as days: adding `TimeDelta::HOUR` moves the wall clock one hour, even across a clock change. If you want to move along the timeline instead, do the sum in UTC, where the two readings never part. See [24 real hours later](#24-real-hours-later).

A `TimeDelta` is built with `TimeDelta::new(days, seconds, microseconds)`, `from_days`, `from_hours`, `from_minutes`, `from_seconds`, `from_micros`, or from the constants `DAY`, `HOUR`, `MINUTE`, `SECOND` and `MICROSECOND`, and combined with `checked_add`, `checked_sub`, `checked_mul` and `checked_neg`.

## When the clocks change

Two things can go wrong with a wall-clock reading, and `fold` settles both.

### An hour that happens twice

When the clocks go back, one hour of wall-clock time is lived through twice. In Madrid on 25 October 2026, 02:30 comes round once in summer time and again, an hour later, in winter time. `fold` says which is meant: off for the first, on for the second.

```rust
use wallclock::{DateTime, Tz};

fn main() -> Result<(), wallclock::Error> {
    let madrid = Tz::zone("Europe/Madrid")?;
    let first = DateTime::new(2026, 10, 25, &madrid)?.with_time(2, 30, 0, 0)?;
    let second = first.with_fold(true);

    assert_eq!(first.isoformat(), "2026-10-25T02:30:00+02:00");
    assert_eq!(second.isoformat(), "2026-10-25T02:30:00+01:00");
    assert_eq!(second.timestamp_micros() - first.timestamp_micros(), 3_600_000_000);
    Ok(())
}
```

### An hour that never happens

When the clocks go forward, an hour is skipped. In Madrid on 29 March 2026 the clocks jump from 02:00 to 03:00, so 02:30 is never shown. A `DateTime` can still hold that reading. With `fold` off it is read by the rules in force before the change, with `fold` on by the rules after:

```rust
use wallclock::{DateTime, Tz};

fn main() -> Result<(), wallclock::Error> {
    let madrid = Tz::zone("Europe/Madrid")?;
    let skipped = DateTime::new(2026, 3, 29, &madrid)?.with_time(2, 30, 0, 0)?;

    assert_eq!(skipped.isoformat(), "2026-03-29T02:30:00+01:00");
    assert_eq!(skipped.with_fold(true).isoformat(), "2026-03-29T02:30:00+02:00");
    Ok(())
}
```

This is PEP 495, the rule Python follows. The practical upshot: a reading built from parts or reached by arithmetic never fails for falling in a skipped or repeated hour. It gets a definite instant. To find out whether a reading is one of these, see [Is this time real?](#is-this-time-real).

The result of arithmetic always has `fold` off. A conversion from another zone turns it on when the result is the second of two readings.

## Comparing two datetimes

There are two kinds of comparison, and they are deliberately different.

- `==` (the derived one) is plain structural equality: the same wall-clock reading, the same zone and the same `fold`. Use it in tests and as a map key.
- `py_cmp` and `py_eq` are Python's `<` and `==`. Use them when you mean "earlier", "later" or "the same moment".

Python's rule has two cases:

- **Same zone:** the wall clocks are compared, and `fold` is ignored.
- **Different zones:** the instants are compared.

```rust
use std::cmp::Ordering;
use wallclock::{DateTime, Tz};

fn main() -> Result<(), wallclock::Error> {
    let madrid = Tz::zone("Europe/Madrid")?;
    let tokyo = Tz::zone("Asia/Tokyo")?;

    let noon_madrid = DateTime::new(2026, 7, 1, &madrid)?.with_time(12, 0, 0, 0)?;
    let evening_tokyo = DateTime::new(2026, 7, 1, &tokyo)?.with_time(19, 0, 0, 0)?;

    // 12:00 in Madrid and 19:00 in Tokyo are the same moment in July.
    assert!(noon_madrid.py_eq(&evening_tokyo));
    assert_eq!(noon_madrid.py_cmp(&evening_tokyo), Ordering::Equal);
    // But they are not the same value.
    assert_ne!(noon_madrid, evening_tokyo);
    Ok(())
}
```

One exception comes from PEP 495: a reading in a repeated or skipped hour is never `py_eq` to a datetime in another zone, because which instant it stands for depends on `fold`.

`since` subtracts by the same rule and gives a `TimeDelta`. For two datetimes in one zone it is the difference between the wall clocks, so it does not count an hour gained or lost to a clock change. For the time that really passed, see [How much time really passed](#how-much-time-really-passed).

## Moving between zones

`astimezone` gives the same instant on another zone's wall clock. `with_tz` does the opposite: it keeps the wall-clock reading and swaps the zone, which is usually a different instant.

```rust
use wallclock::{DateTime, Tz};

fn main() -> Result<(), wallclock::Error> {
    let madrid = Tz::zone("Europe/Madrid")?;
    let new_york = Tz::zone("America/New_York")?;
    let noon = DateTime::new(2026, 7, 1, &madrid)?.with_time(12, 0, 0, 0)?;

    let same_moment = noon.astimezone(&new_york)?;
    assert_eq!(same_moment.isoformat(), "2026-07-01T06:00:00-04:00");

    let same_reading = noon.with_tz(&new_york);
    assert_eq!(same_reading.isoformat(), "2026-07-01T12:00:00-04:00");
    Ok(())
}
```

`timestamp_micros` is the instant as whole microseconds since the Unix epoch, exactly. `timestamp` is the same in seconds as an `f64`, which is what Python's `timestamp()` returns; an `f64` cannot hold every microsecond of a far-off date, so prefer `timestamp_micros` when the value will be compared or stored.

`utcoffset` is how far the wall clock is ahead of UTC at that reading, and `tzname` is the zone's abbreviation there (`CEST`, `EST`, or `UTC+05:30` for a fixed offset).

## Text

`isoformat` writes ISO 8601 as Python does: `2026-10-25T02:30:00+01:00`. Microseconds appear only when they are not zero, and so do seconds in the offset. Printing a `DateTime` with `{}` gives Python's `str()`, which is the same with a space in place of the `T`. A `TimeDelta` prints as Python's does: `1 day, 2:03:04`.

Parsing text into a `DateTime` is not part of this crate.

## Errors and the range of years

A `DateTime` covers years 1 to 9999, like Python's. Anything that would leave that range is an error; nothing is clamped or wrapped (`DateTime::now` aside, which has to return something if the system clock is absurd).

| `Error` | When | Python raises |
|---|---|---|
| `Overflow` | arithmetic or a conversion leaves years 1 to 9999; a `TimeDelta` beyond 999,999,999 days | `OverflowError` |
| `InvalidValue` | a date that does not exist (30 February), a time of day out of range, an offset of 24 hours or more, a malformed zone key | `ValueError` |
| `UnknownZone` | no zone of that name in the database | `ZoneInfoNotFoundError` |

`Error::python_exception()` gives the name in the last column, for code that has to report failures the way a Python program would.

```rust
use wallclock::{DateTime, Error, TimeDelta, Tz};

fn main() {
    let last_day = DateTime::new(9999, 12, 31, &Tz::UTC).unwrap();
    assert_eq!(last_day.checked_add(TimeDelta::DAY), Err(Error::Overflow));

    let no_such_day = DateTime::new(2026, 2, 30, &Tz::UTC).unwrap_err();
    assert_eq!(no_such_day.python_exception(), "ValueError");

    let no_such_zone = Tz::zone("Mars/Olympus_Mons").unwrap_err();
    assert_eq!(no_such_zone.python_exception(), "ZoneInfoNotFoundError");
}
```

## The time zone database

Named zones come from the IANA time zone database, the same data Python's `zoneinfo` uses.

- **Linux, macOS and other Unix systems** have a copy, usually in `/usr/share/zoneinfo`, kept current by system updates. `Tz::zone` reads it.
- **Windows** has none, so a copy is compiled into your program.
- **Minimal containers** (Alpine, distroless, `scratch`) often have none either. There `Tz::zone` fails with `UnknownZone` unless you install the system's `tzdata` package or turn on this crate's `bundled` feature, which compiles the copy in on every platform:

```toml
[dependencies]
wallclock = { version = "0.1", features = ["bundled"] }
```

A compiled-in copy is as old as your build. A system copy is as new as the last system update. Two programs with copies of different dates can disagree about a zone whose government changed the rules in between; that is true of any two programs, Python ones included.

Zone names are matched exactly, as Python matches them: `Europe/Madrid`, not `europe/madrid`.

## Recipes

### The next time a clock shows a given time

"Every day at 08:00": find the first occurrence after `now`.

```rust
use std::cmp::Ordering;
use wallclock::{DateTime, Error, TimeDelta, Tz};

/// The first of `at`, `at` + 1 day, `at` + 2 days, ... that is after `now`.
fn next_daily(at: &DateTime, now: &DateTime) -> Result<DateTime, Error> {
    let mut next = at.clone();
    while next.py_cmp(now) != Ordering::Greater {
        next = next.checked_add(TimeDelta::DAY)?;
    }
    Ok(next)
}

fn main() -> Result<(), Error> {
    let madrid = Tz::zone("Europe/Madrid")?;
    let alarm = DateTime::new(2026, 10, 24, &madrid)?.with_time(8, 0, 0, 0)?;
    // It is 09:00 on the 24th: today's alarm has gone.
    let now = DateTime::new(2026, 10, 24, &madrid)?.with_time(9, 0, 0, 0)?;

    let next = next_daily(&alarm, &now)?;
    assert_eq!(next.isoformat(), "2026-10-25T08:00:00+01:00"); // 08:00, though the day is 25 hours long
    Ok(())
}
```

### 24 real hours later

To move along the timeline instead of the wall clock, do the sum in UTC.

```rust
use wallclock::{DateTime, TimeDelta, Tz};

fn main() -> Result<(), wallclock::Error> {
    let madrid = Tz::zone("Europe/Madrid")?;
    let at = DateTime::new(2026, 10, 24, &madrid)?.with_time(8, 0, 0, 0)?;

    let a_day_of_seconds_later = at
        .astimezone(&Tz::UTC)?
        .checked_add(TimeDelta::DAY)?
        .astimezone(&madrid)?;
    assert_eq!(a_day_of_seconds_later.isoformat(), "2026-10-25T07:00:00+01:00");
    Ok(())
}
```

### How much time really passed

`since` between two datetimes in one zone compares wall clocks. For elapsed time, subtract the instants.

```rust
use wallclock::{DateTime, TimeDelta, Tz};

fn main() -> Result<(), wallclock::Error> {
    let madrid = Tz::zone("Europe/Madrid")?;
    let start = DateTime::new(2026, 10, 24, &madrid)?.with_time(8, 0, 0, 0)?;
    let end = DateTime::new(2026, 10, 25, &madrid)?.with_time(8, 0, 0, 0)?;

    // On the wall clock: exactly one day.
    assert_eq!(end.since(&start), TimeDelta::DAY);

    // On the timeline: 25 hours.
    let elapsed = TimeDelta::from_micros((end.timestamp_micros() - start.timestamp_micros()).into())?;
    assert_eq!(elapsed, TimeDelta::from_hours(25)?);
    Ok(())
}
```

### Is this time real?

A reading in a repeated hour has two instants; a reading in a skipped hour was never shown on any clock. Both can be detected.

```rust
use wallclock::{DateTime, Tz};

/// Whether the wall clock shows this reading twice (the clocks went back over it).
fn is_repeated(reading: &DateTime) -> bool {
    depends_on_fold(reading) && exists(reading)
}

/// Whether the wall clock never shows this reading (the clocks jumped over it).
fn is_skipped(reading: &DateTime) -> bool {
    depends_on_fold(reading) && !exists(reading)
}

fn depends_on_fold(reading: &DateTime) -> bool {
    let other = reading.with_fold(!reading.fold());
    reading.utcoffset() != other.utcoffset()
}

/// A real reading survives a trip to UTC and back; a skipped one comes back changed.
fn exists(reading: &DateTime) -> bool {
    reading
        .astimezone(&Tz::UTC)
        .and_then(|utc| utc.astimezone(reading.tz()))
        .is_ok_and(|back| back.with_fold(false) == reading.with_fold(false))
}

fn main() -> Result<(), wallclock::Error> {
    let madrid = Tz::zone("Europe/Madrid")?;
    let ordinary = DateTime::new(2026, 7, 1, &madrid)?.with_time(2, 30, 0, 0)?;
    let twice = DateTime::new(2026, 10, 25, &madrid)?.with_time(2, 30, 0, 0)?;
    let never = DateTime::new(2026, 3, 29, &madrid)?.with_time(2, 30, 0, 0)?;

    assert!(!is_repeated(&ordinary) && !is_skipped(&ordinary));
    assert!(is_repeated(&twice) && !is_skipped(&twice));
    assert!(is_skipped(&never) && !is_repeated(&never));
    Ok(())
}
```

### The start of a day

Midnight on a date is what `DateTime::new` gives. For "the start of the day this moment falls in", rebuild from the date:

```rust
use wallclock::{DateTime, Tz};

fn main() -> Result<(), wallclock::Error> {
    let madrid = Tz::zone("Europe/Madrid")?;
    let moment = DateTime::new(2026, 10, 25, &madrid)?.with_time(17, 45, 12, 0)?;

    let start_of_day = moment.with_time(0, 0, 0, 0)?.with_fold(false);
    assert_eq!(start_of_day.isoformat(), "2026-10-25T00:00:00+02:00");
    Ok(())
}
```
