//! Datetimes in time zones, with arithmetic on the wall clock: Python's
//! aware `datetime` and `zoneinfo`, for Rust.
//!
//! Python and most Rust libraries disagree about what "add one day" means
//! for a datetime in a time zone. Python moves the wall clock: 08:00 in
//! Madrid plus one day is 08:00 in Madrid the next day, even on the night
//! the clocks change and the day is 23 or 25 hours long. Code ported from
//! Python, or working beside it, has to do the same to get the same
//! answers. This crate does:
//!
//! - [`DateTime`] is an aware `datetime`: a wall-clock reading, a zone and
//!   PEP 495's `fold`.
//! - [`Tz`] is `zoneinfo.ZoneInfo` (an IANA zone such as `Europe/Madrid`)
//!   or `datetime.timezone` (a fixed offset).
//! - [`TimeDelta`] is `timedelta`.
//!
//! ```
//! use wallclock::{DateTime, TimeDelta, Tz};
//!
//! let madrid = Tz::zone("Europe/Madrid")?;
//! // 08:00 on the day before the clocks go back.
//! let at = DateTime::new(2026, 10, 24, &madrid)?.with_time(8, 0, 0, 0)?;
//! let next = at.checked_add(TimeDelta::DAY)?;
//!
//! // Still 08:00 on the wall clock...
//! assert_eq!(next.isoformat(), "2026-10-25T08:00:00+01:00");
//! // ...which is 25 hours later.
//! assert_eq!(next.timestamp_micros() - at.timestamp_micros(), 25 * 3_600_000_000);
//! # Ok::<(), wallclock::Error>(())
//! ```
//!
//! # What follows Python
//!
//! - **Arithmetic** ([`DateTime::checked_add`]) moves the wall clock and
//!   keeps the zone. The result has `fold` off.
//! - **Repeated and skipped times.** When the clocks go back, an hour of
//!   wall-clock time happens twice; [`DateTime::fold`] picks the second.
//!   When they go forward an hour is skipped; a reading in it is taken by
//!   the rules before the change, or with `fold` by those after.
//! - **Comparison and subtraction** ([`DateTime::py_cmp`],
//!   [`DateTime::py_eq`], [`DateTime::since`]) go by the wall clock for two
//!   datetimes in the same zone and by the instant otherwise, with PEP
//!   495's exception for `==`.
//! - **Conversion** ([`DateTime::astimezone`],
//!   [`DateTime::from_timestamp_micros`]) sets `fold` on the second of two
//!   readings.
//! - **Range.** Years 1 to 9999; going outside is an error, as in Python,
//!   never a wrapped or clamped value (`now` aside).
//! - **`timestamp()`** is the nearest `f64`, as Python's is;
//!   [`DateTime::timestamp_micros`] is exact.
//! - **Text.** [`DateTime::isoformat`], `Display` (Python's `str`) and
//!   [`DateTime::tzname`].
//!
//! Every one of these is checked against answers recorded from CPython
//! (`tests/vectors`).
//!
//! # What is not here
//!
//! Naive datetimes, parsing (`fromisoformat`, `strptime`), `strftime`,
//! `dst()`, and dates and times on their own. Errors carry the name of
//! Python's exception ([`Error::python_exception`]) but not its message.
//!
//! # The time zone database
//!
//! [`Tz::zone`] reads the system's IANA database (`/usr/share/zoneinfo`)
//! where there is one, as Python does. Where there is none (Windows) a copy
//! compiled into the program is used; Python needs its `tzdata` package
//! there. The `bundled` feature compiles the copy in everywhere. The
//! database is [jiff](https://docs.rs/jiff)'s, which reads the rule each
//! zone ends with, so daylight saving keeps turning up in year 9000 as it
//! does in Python.
//!
//! Two databases of different dates can disagree about a zone whose rules
//! changed between them.

// The Rust examples in the README and the guides are tests: `cargo test`
// compiles and runs them.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeExamples;
#[cfg(doctest)]
#[doc = include_str!("../docs/guide.md")]
struct GuideExamples;
#[cfg(doctest)]
#[doc = include_str!("../docs/from-python.md")]
struct FromPythonExamples;

mod civil;
mod datetime;
mod error;
mod timedelta;
mod tz;

pub use datetime::DateTime;
pub use error::Error;
pub use timedelta::TimeDelta;
pub use tz::Tz;
