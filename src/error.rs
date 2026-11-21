//! What can go wrong, named after the exception Python raises for it.

use std::fmt;

/// An error from this crate. Each is the counterpart of a Python exception;
/// [`Error::python_exception`] gives its class name.
///
/// This enum is marked `#[non_exhaustive]` so a later release can add a
/// variant without breaking callers.
///
/// # Examples
///
/// ```
/// use wallclock::{DateTime, Error, TimeDelta, Tz};
///
/// let last = DateTime::new(9999, 12, 31, &Tz::UTC)?;
/// assert_eq!(last.checked_add(TimeDelta::DAY), Err(Error::Overflow));
/// assert!(matches!(DateTime::new(2026, 2, 30, &Tz::UTC), Err(Error::InvalidValue(_))));
/// assert!(matches!(Tz::zone("Mars/Olympus_Mons"), Err(Error::UnknownZone(_))));
/// # Ok::<(), wallclock::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// A result outside what the type holds: a datetime before year 1 or
    /// after year 9999, or a timedelta beyond 999,999,999 days. Python's
    /// `OverflowError`.
    Overflow,
    /// A field that is not a calendar date or a time of day, a timestamp
    /// outside years 1 to 9999, an offset of 24 hours or more, or a
    /// malformed zone key. Python's `ValueError`.
    InvalidValue(&'static str),
    /// No time zone of that name in the database. Python's
    /// `ZoneInfoNotFoundError`.
    UnknownZone(String),
}

impl Error {
    /// The class name of the exception Python raises here.
    ///
    /// # Examples
    ///
    /// ```
    /// use wallclock::{DateTime, Error, Tz};
    ///
    /// assert_eq!(Error::Overflow.python_exception(), "OverflowError");
    /// assert_eq!(DateTime::new(2026, 2, 30, &Tz::UTC).unwrap_err().python_exception(), "ValueError");
    /// assert_eq!(Tz::zone("Mars/Olympus_Mons").unwrap_err().python_exception(), "ZoneInfoNotFoundError");
    /// ```
    pub fn python_exception(&self) -> &'static str {
        match self {
            Self::Overflow => "OverflowError",
            Self::InvalidValue(_) => "ValueError",
            Self::UnknownZone(_) => "ZoneInfoNotFoundError",
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Overflow => f.write_str("date value out of range"),
            Self::InvalidValue(what) => f.write_str(what),
            Self::UnknownZone(key) => write!(f, "No time zone found with key {key}"),
        }
    }
}

impl std::error::Error for Error {}
