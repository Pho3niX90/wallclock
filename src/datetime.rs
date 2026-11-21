//! Python's aware `datetime`.

use std::cmp::Ordering;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::civil::{self, Fields};
use crate::error::Error;
use crate::timedelta::{TimeDelta, divide};
use crate::tz::{Tz, offset_text};

/// A date and time of day in a time zone: Python's aware `datetime`.
///
/// Like Python's, it is a wall-clock reading (year to microsecond), the
/// zone, and `fold`, which says which of two readings is meant where a
/// clock change makes the wall-clock time happen twice or not at all. The
/// instant follows from the three.
///
/// The derived `==` compares those three parts. Python's own `==` and `<`
/// are [`DateTime::py_eq`] and [`DateTime::py_cmp`].
///
/// # Examples
///
/// ```
/// use wallclock::{DateTime, TimeDelta, Tz};
///
/// let madrid = Tz::zone("Europe/Madrid")?;
/// let morning = DateTime::new(2026, 10, 24, &madrid)?.with_time(8, 0, 0, 0)?;
/// let next = morning.checked_add(TimeDelta::DAY)?;
/// assert_eq!(next.isoformat(), "2026-10-25T08:00:00+01:00");
/// # Ok::<(), wallclock::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DateTime {
    /// Microseconds from 1970-01-01T00:00:00 on the wall clock.
    wall: i64,
    fold: bool,
    tz: Tz,
}

impl DateTime {
    /// `datetime(year, month, day, tzinfo=tz)`: midnight on that date.
    ///
    /// Fails with [`Error::InvalidValue`] outside years 1 to 9999 or for a
    /// day the month does not have.
    ///
    /// # Examples
    ///
    /// ```
    /// use wallclock::{DateTime, Error, Tz};
    ///
    /// let madrid = Tz::zone("Europe/Madrid")?;
    /// let day = DateTime::new(2026, 7, 1, &madrid)?;
    /// assert_eq!(day.isoformat(), "2026-07-01T00:00:00+02:00");
    /// assert!(matches!(DateTime::new(2026, 2, 29, &Tz::UTC), Err(Error::InvalidValue(_))));
    /// # Ok::<(), wallclock::Error>(())
    /// ```
    pub fn new(year: i32, month: u32, day: u32, tz: &Tz) -> Result<Self, Error> {
        if !(1..=9999).contains(&year) {
            return Err(Error::InvalidValue("year is out of range"));
        }
        if !(1..=12).contains(&month) {
            return Err(Error::InvalidValue("month must be in 1..12"));
        }
        if day < 1 || day > civil::days_in_month(year, month) {
            return Err(Error::InvalidValue("day is out of range for month"));
        }
        let fields = Fields {
            year,
            month,
            day,
            hour: 0,
            minute: 0,
            second: 0,
            microsecond: 0,
        };
        Ok(Self {
            wall: civil::from_fields(fields),
            fold: false,
            tz: tz.clone(),
        })
    }

    /// `.replace(hour=..., minute=..., second=..., microsecond=...)`: the
    /// same date at another time of day. `fold` is kept.
    ///
    /// # Examples
    ///
    /// ```
    /// use wallclock::{DateTime, Tz};
    ///
    /// let new_york = Tz::zone("America/New_York")?;
    /// let day = DateTime::new(2026, 7, 1, &new_york)?;
    /// let at = day.with_time(14, 30, 0, 500_000)?;
    /// assert_eq!(at.isoformat(), "2026-07-01T14:30:00.500000-04:00");
    /// # Ok::<(), wallclock::Error>(())
    /// ```
    pub fn with_time(
        &self,
        hour: u32,
        minute: u32,
        second: u32,
        microsecond: u32,
    ) -> Result<Self, Error> {
        if hour > 23 {
            return Err(Error::InvalidValue("hour must be in 0..23"));
        }
        if minute > 59 {
            return Err(Error::InvalidValue("minute must be in 0..59"));
        }
        if second > 59 {
            return Err(Error::InvalidValue("second must be in 0..59"));
        }
        if microsecond > 999_999 {
            return Err(Error::InvalidValue("microsecond must be in 0..999999"));
        }
        let fields = Fields {
            hour,
            minute,
            second,
            microsecond,
            ..civil::to_fields(self.wall)
        };
        Ok(Self {
            wall: civil::from_fields(fields),
            ..self.clone()
        })
    }

    /// `.replace(fold=fold)`: the other reading of a wall-clock time that a
    /// clock change repeats or skips.
    ///
    /// # Examples
    ///
    /// ```
    /// use wallclock::{DateTime, Tz};
    ///
    /// let madrid = Tz::zone("Europe/Madrid")?;
    /// let first = DateTime::new(2026, 10, 25, &madrid)?.with_time(2, 30, 0, 0)?;
    /// let second = first.with_fold(true);
    ///
    /// assert_eq!(first.isoformat(), "2026-10-25T02:30:00+02:00");
    /// assert_eq!(second.isoformat(), "2026-10-25T02:30:00+01:00");
    /// # Ok::<(), wallclock::Error>(())
    /// ```
    #[must_use]
    pub fn with_fold(&self, fold: bool) -> Self {
        Self {
            wall: self.wall,
            fold,
            tz: self.tz.clone(),
        }
    }

    /// `.replace(tzinfo=tz)`: the same wall-clock reading and `fold` in
    /// another zone. This is usually a different instant; to keep the
    /// instant, use [`DateTime::astimezone`].
    ///
    /// # Examples
    ///
    /// ```
    /// use wallclock::{DateTime, Tz};
    ///
    /// let madrid = Tz::zone("Europe/Madrid")?;
    /// let new_york = Tz::zone("America/New_York")?;
    /// let noon = DateTime::new(2026, 7, 1, &madrid)?.with_time(12, 0, 0, 0)?;
    ///
    /// let relabelled = noon.with_tz(&new_york);
    /// assert_eq!(relabelled.isoformat(), "2026-07-01T12:00:00-04:00");
    /// # Ok::<(), wallclock::Error>(())
    /// ```
    #[must_use]
    pub fn with_tz(&self, tz: &Tz) -> Self {
        Self {
            wall: self.wall,
            fold: self.fold,
            tz: tz.clone(),
        }
    }

    /// `datetime.fromtimestamp(t, tz)` for `t` given exactly, as
    /// microseconds from the Unix epoch.
    ///
    /// Fails with [`Error::InvalidValue`] outside years 1 to 9999 in UTC,
    /// and with [`Error::Overflow`] where the zone's offset takes the
    /// reading past either end.
    ///
    /// # Examples
    ///
    /// ```
    /// use wallclock::{DateTime, Tz};
    ///
    /// let kolkata = Tz::zone("Asia/Kolkata")?;
    /// let at = DateTime::from_timestamp_micros(1_782_907_200_000_000, &kolkata)?;
    /// assert_eq!(at.isoformat(), "2026-07-01T17:30:00+05:30");
    /// # Ok::<(), wallclock::Error>(())
    /// ```
    pub fn from_timestamp_micros(micros: i64, tz: &Tz) -> Result<Self, Error> {
        if !(civil::MIN..=civil::MAX).contains(&micros) {
            return Err(Error::InvalidValue("year is out of range"));
        }
        from_utc(micros, tz)
    }

    /// `datetime.now(tz)`: the system clock's time in `tz`.
    ///
    /// A system clock outside years 1 to 9999 gives the nearest end of the
    /// calendar.
    ///
    /// # Examples
    ///
    /// ```
    /// use wallclock::{DateTime, Tz};
    ///
    /// let now = DateTime::now(&Tz::UTC);
    /// assert_eq!(now.tz(), &Tz::UTC);
    /// assert!((1..=9999).contains(&now.year()));
    /// ```
    pub fn now(tz: &Tz) -> Self {
        let micros = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(after) => i64::try_from(after.as_micros()).unwrap_or(i64::MAX),
            Err(before) => {
                i64::try_from(before.duration().as_micros()).map_or(i64::MIN, |micros| -micros)
            }
        };
        let micros = micros.clamp(civil::MIN, civil::MAX);
        from_utc(micros, tz).unwrap_or_else(|_| {
            // A clock at the very end of the calendar, in a zone that
            // reaches past it: the end itself.
            let wall = if micros < 0 { civil::MIN } else { civil::MAX };
            Self {
                wall,
                fold: false,
                tz: tz.clone(),
            }
        })
    }

    fn fields(&self) -> Fields {
        civil::to_fields(self.wall)
    }

    /// `.year`, 1 to 9999.
    ///
    /// # Examples
    ///
    /// ```
    /// use wallclock::{DateTime, Tz};
    ///
    /// let tokyo = Tz::zone("Asia/Tokyo")?;
    /// let at = DateTime::new(2026, 7, 1, &tokyo)?.with_time(9, 15, 30, 250_000)?;
    ///
    /// assert_eq!((at.year(), at.month(), at.day()), (2026, 7, 1));
    /// assert_eq!((at.hour(), at.minute(), at.second(), at.microsecond()), (9, 15, 30, 250_000));
    /// # Ok::<(), wallclock::Error>(())
    /// ```
    pub fn year(&self) -> i32 {
        self.fields().year
    }

    /// `.month`, 1 to 12. See [`DateTime::year`] for an example.
    pub fn month(&self) -> u32 {
        self.fields().month
    }

    /// `.day`, 1 to 31. See [`DateTime::year`] for an example.
    pub fn day(&self) -> u32 {
        self.fields().day
    }

    /// `.hour`, 0 to 23. See [`DateTime::year`] for an example.
    pub fn hour(&self) -> u32 {
        self.fields().hour
    }

    /// `.minute`, 0 to 59. See [`DateTime::year`] for an example.
    pub fn minute(&self) -> u32 {
        self.fields().minute
    }

    /// `.second`, 0 to 59. See [`DateTime::year`] for an example.
    pub fn second(&self) -> u32 {
        self.fields().second
    }

    /// `.microsecond`, 0 to 999999. See [`DateTime::year`] for an example.
    pub fn microsecond(&self) -> u32 {
        self.fields().microsecond
    }

    /// `.weekday()`: Monday is 0, Sunday is 6.
    ///
    /// # Examples
    ///
    /// ```
    /// use wallclock::{DateTime, Tz};
    ///
    /// // 25 October 2026 is a Sunday.
    /// let sunday = DateTime::new(2026, 10, 25, &Tz::UTC)?;
    /// assert_eq!(sunday.weekday(), 6);
    /// # Ok::<(), wallclock::Error>(())
    /// ```
    pub fn weekday(&self) -> u32 {
        civil::weekday(self.wall)
    }

    /// `.fold`.
    ///
    /// # Examples
    ///
    /// ```
    /// use wallclock::{DateTime, Tz};
    ///
    /// let madrid = Tz::zone("Europe/Madrid")?;
    /// let first = DateTime::new(2026, 10, 25, &madrid)?.with_time(2, 30, 0, 0)?;
    /// let second = first.with_fold(true);
    ///
    /// assert!(!first.fold());
    /// assert!(second.fold());
    /// # Ok::<(), wallclock::Error>(())
    /// ```
    pub fn fold(&self) -> bool {
        self.fold
    }

    /// `.tzinfo`.
    ///
    /// # Examples
    ///
    /// ```
    /// use wallclock::{DateTime, Tz};
    ///
    /// let madrid = Tz::zone("Europe/Madrid")?;
    /// let day = DateTime::new(2026, 7, 1, &madrid)?;
    /// assert_eq!(day.tz(), &madrid);
    /// assert_eq!(day.tz().key(), Some("Europe/Madrid"));
    /// # Ok::<(), wallclock::Error>(())
    /// ```
    pub fn tz(&self) -> &Tz {
        &self.tz
    }

    /// `.utcoffset()`: how far the wall clock is ahead of UTC.
    ///
    /// # Examples
    ///
    /// ```
    /// use wallclock::{DateTime, TimeDelta, Tz};
    ///
    /// let madrid = Tz::zone("Europe/Madrid")?;
    /// // 02:30 on 29 March 2026 is skipped: fold picks the rules before or after.
    /// let skipped = DateTime::new(2026, 3, 29, &madrid)?.with_time(2, 30, 0, 0)?;
    /// assert_eq!(skipped.utcoffset(), TimeDelta::HOUR);
    /// assert_eq!(skipped.with_fold(true).utcoffset(), TimeDelta::from_hours(2)?);
    /// # Ok::<(), wallclock::Error>(())
    /// ```
    pub fn utcoffset(&self) -> TimeDelta {
        TimeDelta::from_micros(i128::from(self.offset())).expect("an offset is under a day")
    }

    fn offset(&self) -> i64 {
        self.tz.local(self.wall, self.fold).offset
    }

    /// `.tzname()`: the zone's abbreviation at this time, such as `CEST`,
    /// or `UTC+02:00` for a fixed offset.
    ///
    /// # Examples
    ///
    /// ```
    /// use wallclock::{DateTime, Tz};
    ///
    /// let madrid = Tz::zone("Europe/Madrid")?;
    /// let india = Tz::fixed(5 * 3600 + 30 * 60)?;
    /// assert_eq!(DateTime::new(2026, 7, 1, &madrid)?.tzname(), "CEST");
    /// assert_eq!(DateTime::new(2026, 1, 15, &madrid)?.tzname(), "CET");
    /// assert_eq!(DateTime::new(2026, 7, 1, &india)?.tzname(), "UTC+05:30");
    /// # Ok::<(), wallclock::Error>(())
    /// ```
    pub fn tzname(&self) -> String {
        self.tz.name(self.wall, self.fold)
    }

    /// The instant, exactly, as microseconds from the Unix epoch.
    ///
    /// # Examples
    ///
    /// ```
    /// use wallclock::{DateTime, Tz};
    ///
    /// let madrid = Tz::zone("Europe/Madrid")?;
    /// let first = DateTime::new(2026, 10, 25, &madrid)?.with_time(2, 30, 0, 0)?;
    /// let second = first.with_fold(true);
    /// assert_eq!(second.timestamp_micros() - first.timestamp_micros(), 3_600_000_000);
    /// # Ok::<(), wallclock::Error>(())
    /// ```
    pub fn timestamp_micros(&self) -> i64 {
        self.wall - self.offset()
    }

    /// `.timestamp()`: the instant in seconds from the Unix epoch, as the
    /// nearest `f64`. Not every microsecond survives that; for the exact
    /// instant use [`DateTime::timestamp_micros`].
    ///
    /// # Examples
    ///
    /// ```
    /// use wallclock::{DateTime, Tz};
    ///
    /// let at = DateTime::new(2026, 7, 1, &Tz::UTC)?.with_time(12, 0, 0, 500_000)?;
    /// assert_eq!(at.timestamp(), 1_782_907_200.5);
    /// # Ok::<(), wallclock::Error>(())
    /// ```
    pub fn timestamp(&self) -> f64 {
        divide(i128::from(self.timestamp_micros()), 1_000_000)
    }

    /// `self + delta`: the wall clock moved by `delta`, in the same zone.
    ///
    /// This is Python's arithmetic. It moves the clock reading, not the
    /// instant, so a day added to 08:00 is 08:00 the next day even when a
    /// clock change made that day 23 or 25 hours long. To move the instant,
    /// go through [`DateTime::astimezone`] to UTC and back. The result has
    /// `fold` off.
    ///
    /// Fails with [`Error::Overflow`] before year 1 or after year 9999.
    ///
    /// # Examples
    ///
    /// ```
    /// use wallclock::{DateTime, TimeDelta, Tz};
    ///
    /// let madrid = Tz::zone("Europe/Madrid")?;
    /// let before = DateTime::new(2026, 10, 24, &madrid)?.with_time(8, 0, 0, 0)?;
    /// let after = before.checked_add(TimeDelta::DAY)?;
    ///
    /// assert_eq!(after.isoformat(), "2026-10-25T08:00:00+01:00");
    /// assert_eq!(after.timestamp_micros() - before.timestamp_micros(), 25 * 3_600_000_000);
    /// # Ok::<(), wallclock::Error>(())
    /// ```
    pub fn checked_add(&self, delta: TimeDelta) -> Result<Self, Error> {
        let wall = i128::from(self.wall) + delta.total_microseconds();
        let wall = i64::try_from(wall).map_err(|_| Error::Overflow)?;
        if !(civil::MIN..=civil::MAX).contains(&wall) {
            return Err(Error::Overflow);
        }
        Ok(Self {
            wall,
            fold: false,
            tz: self.tz.clone(),
        })
    }

    /// `self - delta`: see [`DateTime::checked_add`].
    ///
    /// # Examples
    ///
    /// ```
    /// use wallclock::{DateTime, TimeDelta, Tz};
    ///
    /// let madrid = Tz::zone("Europe/Madrid")?;
    /// let after = DateTime::new(2026, 3, 29, &madrid)?.with_time(8, 0, 0, 0)?;
    /// let before = after.checked_sub(TimeDelta::DAY)?;
    ///
    /// assert_eq!(before.isoformat(), "2026-03-28T08:00:00+01:00");
    /// assert_eq!(after.timestamp_micros() - before.timestamp_micros(), 23 * 3_600_000_000);
    /// # Ok::<(), wallclock::Error>(())
    /// ```
    pub fn checked_sub(&self, delta: TimeDelta) -> Result<Self, Error> {
        self.checked_add(delta.checked_neg()?)
    }

    /// `self - earlier`: the time between two datetimes.
    ///
    /// As in Python, two datetimes in the same zone are subtracted by their
    /// wall clocks, ignoring `fold` and any clock change between them;
    /// datetimes in different zones by their instants.
    ///
    /// # Examples
    ///
    /// ```
    /// use wallclock::{DateTime, TimeDelta, Tz};
    ///
    /// let madrid = Tz::zone("Europe/Madrid")?;
    /// let start = DateTime::new(2026, 10, 24, &madrid)?.with_time(8, 0, 0, 0)?;
    /// let end = DateTime::new(2026, 10, 25, &madrid)?.with_time(8, 0, 0, 0)?;
    ///
    /// assert_eq!(end.since(&start), TimeDelta::DAY);
    /// assert_eq!(end.astimezone(&Tz::UTC)?.since(&start), TimeDelta::from_hours(25)?);
    /// # Ok::<(), wallclock::Error>(())
    /// ```
    pub fn since(&self, earlier: &Self) -> TimeDelta {
        let micros = if self.tz == earlier.tz {
            i128::from(self.wall) - i128::from(earlier.wall)
        } else {
            i128::from(self.timestamp_micros()) - i128::from(earlier.timestamp_micros())
        };
        TimeDelta::from_micros(micros).expect("two datetimes are under ten thousand years apart")
    }

    /// Python's `<`, `<=`, `>` and `>=`: two datetimes in the same zone are
    /// ordered by their wall clocks (ignoring `fold`), datetimes in
    /// different zones by their instants.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::cmp::Ordering;
    /// use wallclock::{DateTime, Tz};
    ///
    /// let madrid = Tz::zone("Europe/Madrid")?;
    /// let new_york = Tz::zone("America/New_York")?;
    /// let noon_madrid = DateTime::new(2026, 7, 1, &madrid)?.with_time(12, 0, 0, 0)?;
    /// let morning_ny = DateTime::new(2026, 7, 1, &new_york)?.with_time(8, 0, 0, 0)?;
    ///
    /// // 12:00 CEST (10:00 UTC) is earlier than 08:00 EDT (12:00 UTC).
    /// assert_eq!(noon_madrid.py_cmp(&morning_ny), Ordering::Less);
    /// # Ok::<(), wallclock::Error>(())
    /// ```
    pub fn py_cmp(&self, other: &Self) -> Ordering {
        if self.tz == other.tz {
            self.wall.cmp(&other.wall)
        } else {
            self.timestamp_micros().cmp(&other.timestamp_micros())
        }
    }

    /// Python's `==`: two datetimes in the same zone are equal when their
    /// wall clocks are (ignoring `fold`); in different zones when they are
    /// the same instant, except that a wall-clock time a clock change
    /// repeats or skips equals nothing in another zone (PEP 495).
    ///
    /// # Examples
    ///
    /// ```
    /// use wallclock::{DateTime, Tz};
    ///
    /// let madrid = Tz::zone("Europe/Madrid")?;
    /// let tokyo = Tz::zone("Asia/Tokyo")?;
    /// let noon_madrid = DateTime::new(2026, 7, 1, &madrid)?.with_time(12, 0, 0, 0)?;
    /// let evening_tokyo = DateTime::new(2026, 7, 1, &tokyo)?.with_time(19, 0, 0, 0)?;
    ///
    /// assert!(noon_madrid.py_eq(&evening_tokyo));
    /// assert_ne!(noon_madrid, evening_tokyo);
    /// # Ok::<(), wallclock::Error>(())
    /// ```
    pub fn py_eq(&self, other: &Self) -> bool {
        if self.tz == other.tz {
            return self.wall == other.wall;
        }
        let depends_on_fold = |datetime: &Self| {
            let local = datetime.tz.local(datetime.wall, datetime.fold);
            local.other != local.offset
        };
        !depends_on_fold(self)
            && !depends_on_fold(other)
            && self.timestamp_micros() == other.timestamp_micros()
    }

    /// `.astimezone(tz)`: the same instant on the wall clock of `tz`. Where
    /// that reading happens twice, `fold` says which one this is.
    ///
    /// Converting to the zone the datetime is already in gives it back
    /// unchanged, `fold` included. Python does that for the same `tzinfo`
    /// object, which two `ZoneInfo`s of one key are, and UTC, of which
    /// there is one. Two equal `timezone`s of another offset are not, so
    /// for those the conversion is carried out.
    ///
    /// Fails with [`Error::Overflow`] where the instant, in UTC or in `tz`,
    /// is before year 1 or after year 9999.
    ///
    /// # Examples
    ///
    /// ```
    /// use wallclock::{DateTime, Tz};
    ///
    /// let madrid = Tz::zone("Europe/Madrid")?;
    /// // 01:30 UTC is the second 02:30 in Madrid on the night the clocks go back.
    /// let utc = DateTime::new(2026, 10, 25, &Tz::UTC)?.with_time(1, 30, 0, 0)?;
    /// let local = utc.astimezone(&madrid)?;
    ///
    /// assert_eq!(local.isoformat(), "2026-10-25T02:30:00+01:00");
    /// assert!(local.fold());
    /// # Ok::<(), wallclock::Error>(())
    /// ```
    pub fn astimezone(&self, tz: &Tz) -> Result<Self, Error> {
        if *tz == self.tz && (tz.key().is_some() || *tz == Tz::UTC) {
            return Ok(self.clone());
        }
        let utc = self.timestamp_micros();
        if !(civil::MIN..=civil::MAX).contains(&utc) {
            return Err(Error::Overflow);
        }
        from_utc(utc, tz)
    }

    /// `.isoformat()`: `2026-10-25T02:30:00+01:00`, with microseconds, and
    /// seconds in the offset, only where they are not zero.
    ///
    /// # Examples
    ///
    /// ```
    /// use wallclock::{DateTime, Tz};
    ///
    /// let kolkata = Tz::zone("Asia/Kolkata")?;
    /// let whole = DateTime::new(2026, 7, 1, &kolkata)?.with_time(12, 0, 0, 0)?;
    /// let exact = DateTime::new(2026, 7, 1, &kolkata)?.with_time(12, 0, 0, 42)?;
    ///
    /// assert_eq!(whole.isoformat(), "2026-07-01T12:00:00+05:30");
    /// assert_eq!(exact.isoformat(), "2026-07-01T12:00:00.000042+05:30");
    /// # Ok::<(), wallclock::Error>(())
    /// ```
    pub fn isoformat(&self) -> String {
        self.format('T')
    }

    fn format(&self, separator: char) -> String {
        let fields = self.fields();
        let mut text = format!(
            "{:04}-{:02}-{:02}{separator}{:02}:{:02}:{:02}",
            fields.year, fields.month, fields.day, fields.hour, fields.minute, fields.second
        );
        if fields.microsecond != 0 {
            text.push_str(&format!(".{:06}", fields.microsecond));
        }
        text.push_str(&offset_text(self.offset()));
        text
    }
}

/// `tz.fromutc()`: the reading in `tz` of the instant `utc`.
fn from_utc(utc: i64, tz: &Tz) -> Result<DateTime, Error> {
    let offset = tz.offset_at(utc);
    let wall = utc + offset;
    if !(civil::MIN..=civil::MAX).contains(&wall) {
        return Err(Error::Overflow);
    }
    // The second of two readings: the time is ambiguous and this instant
    // has the later offset, which is the one `fold` selects.
    let second = tz.local(wall, true);
    let fold = second.ambiguous && second.offset == offset;
    Ok(DateTime {
        wall,
        fold,
        tz: tz.clone(),
    })
}

/// Python's `str(datetime)`: `isoformat()` with a space for the `T`.
///
/// # Examples
///
/// ```
/// use wallclock::{DateTime, Tz};
///
/// let madrid = Tz::zone("Europe/Madrid")?;
/// let at = DateTime::new(2026, 10, 25, &madrid)?.with_time(8, 0, 0, 0)?;
/// assert_eq!(at.to_string(), "2026-10-25 08:00:00+01:00");
/// # Ok::<(), wallclock::Error>(())
/// ```
impl fmt::Display for DateTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.format(' '))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn madrid() -> Tz {
        Tz::zone("Europe/Madrid").unwrap()
    }

    fn at(year: i32, month: u32, day: u32, hour: u32, minute: u32, tz: &Tz) -> DateTime {
        DateTime::new(year, month, day, tz)
            .unwrap()
            .with_time(hour, minute, 0, 0)
            .unwrap()
    }

    #[test]
    fn fields_are_checked_as_python_checks_them() {
        let invalid = |result: Result<DateTime, Error>| result.unwrap_err().python_exception();
        assert_eq!(invalid(DateTime::new(0, 1, 1, &Tz::UTC)), "ValueError");
        assert_eq!(invalid(DateTime::new(10_000, 1, 1, &Tz::UTC)), "ValueError");
        assert_eq!(invalid(DateTime::new(2026, 0, 1, &Tz::UTC)), "ValueError");
        assert_eq!(invalid(DateTime::new(2026, 13, 1, &Tz::UTC)), "ValueError");
        assert_eq!(invalid(DateTime::new(2026, 2, 29, &Tz::UTC)), "ValueError");
        assert_eq!(invalid(DateTime::new(2026, 4, 0, &Tz::UTC)), "ValueError");
        let day = DateTime::new(2024, 2, 29, &Tz::UTC).unwrap();
        assert_eq!(invalid(day.with_time(24, 0, 0, 0)), "ValueError");
        assert_eq!(invalid(day.with_time(0, 60, 0, 0)), "ValueError");
        assert_eq!(invalid(day.with_time(0, 0, 60, 0)), "ValueError");
        assert_eq!(invalid(day.with_time(0, 0, 0, 1_000_000)), "ValueError");
        let late = day.with_time(23, 59, 59, 999_999).unwrap();
        assert_eq!(
            (late.year(), late.month(), late.day(), late.hour()),
            (2024, 2, 29, 23)
        );
        assert_eq!(
            (
                late.minute(),
                late.second(),
                late.microsecond(),
                late.weekday()
            ),
            (59, 59, 999_999, 3)
        );
        assert_eq!(late.to_string(), "2024-02-29 23:59:59.999999+00:00");
    }

    #[test]
    fn a_day_added_keeps_the_time_of_day() {
        // The clocks go back in the night of 24 to 25 October 2026.
        let eight = at(2026, 10, 24, 8, 0, &madrid());
        let next = eight.checked_add(TimeDelta::DAY).unwrap();
        assert_eq!(next.isoformat(), "2026-10-25T08:00:00+01:00");
        assert_eq!(
            next.timestamp_micros() - eight.timestamp_micros(),
            25 * 3_600_000_000
        );
        // In the same zone the difference is the wall clock's; across zones, the instants'.
        assert_eq!(next.since(&eight), TimeDelta::DAY);
        let utc = next.astimezone(&Tz::UTC).unwrap();
        assert_eq!(utc.since(&eight).total_microseconds(), 25 * 3_600_000_000);
        assert_eq!(next.checked_sub(TimeDelta::DAY).unwrap(), eight);
    }

    #[test]
    fn a_repeated_hour_has_two_readings() {
        let madrid = madrid();
        let first = at(2026, 10, 25, 2, 30, &madrid);
        let second = first.with_fold(true);
        assert_eq!(
            (first.tzname(), second.tzname()),
            ("CEST".into(), "CET".into())
        );
        assert_eq!(
            second.timestamp_micros() - first.timestamp_micros(),
            3_600_000_000
        );
        // The same zone: the wall clocks are equal, whatever `fold` says.
        assert!(first.py_eq(&second));
        assert_eq!(first.py_cmp(&second), Ordering::Equal);
        assert_ne!(first, second);
        // Another zone: ordered by instant, and such a reading equals nothing.
        let same_instant = second.astimezone(&Tz::UTC).unwrap();
        assert_eq!(same_instant.isoformat(), "2026-10-25T01:30:00+00:00");
        assert_eq!(second.py_cmp(&same_instant), Ordering::Equal);
        assert!(!second.py_eq(&same_instant));
        assert_eq!(first.py_cmp(&same_instant), Ordering::Less);
        // Coming back from UTC finds the second reading again.
        assert_eq!(same_instant.astimezone(&madrid).unwrap(), second);
        assert!(same_instant.astimezone(&madrid).unwrap().fold());
    }

    #[test]
    fn a_skipped_hour_is_read_by_the_rules_on_either_side() {
        let before = at(2026, 3, 29, 2, 30, &madrid());
        let after = before.with_fold(true);
        assert_eq!(before.isoformat(), "2026-03-29T02:30:00+01:00");
        assert_eq!(after.isoformat(), "2026-03-29T02:30:00+02:00");
        assert_eq!(
            (before.tzname(), after.tzname()),
            ("CET".into(), "CEST".into())
        );
        assert_eq!(before.utcoffset(), TimeDelta::HOUR);
    }

    #[test]
    fn the_ends_of_the_calendar_are_errors() {
        let last = DateTime::new(9999, 12, 31, &Tz::UTC).unwrap();
        assert_eq!(last.checked_add(TimeDelta::DAY), Err(Error::Overflow));
        assert_eq!(last.checked_add(TimeDelta::MAX), Err(Error::Overflow));
        assert_eq!(last.checked_sub(TimeDelta::MAX), Err(Error::Overflow));
        let first = DateTime::new(1, 1, 1, &Tz::UTC).unwrap();
        assert_eq!(
            first.checked_sub(TimeDelta::MICROSECOND),
            Err(Error::Overflow)
        );
        assert_eq!(first.checked_add(TimeDelta::MIN), Err(Error::Overflow));
        // The instant exists in UTC but not on a clock five hours behind.
        let west = Tz::fixed(-5 * 3600).unwrap();
        assert_eq!(first.astimezone(&west), Err(Error::Overflow));
        assert_eq!(
            DateTime::from_timestamp_micros(first.timestamp_micros(), &west),
            Err(Error::Overflow)
        );
        // And a reading whose instant is before year 1 cannot be moved at all.
        let east = Tz::fixed(3600).unwrap();
        let early = DateTime::new(1, 1, 1, &east).unwrap();
        assert_eq!(early.astimezone(&Tz::UTC), Err(Error::Overflow));
        assert_eq!(early.timestamp(), -62_135_600_400.0);
        let outside = DateTime::from_timestamp_micros(i64::MAX, &Tz::UTC).unwrap_err();
        assert_eq!(outside.python_exception(), "ValueError");
        assert!(DateTime::from_timestamp_micros(i64::MIN, &west).is_err());
    }

    #[test]
    fn replacing_the_zone_keeps_the_reading() {
        let madrid = madrid();
        let reading = at(2026, 7, 1, 12, 0, &madrid).with_fold(true);
        let elsewhere = reading.with_tz(&Tz::UTC);
        assert_eq!(elsewhere.isoformat(), "2026-07-01T12:00:00+00:00");
        assert!(elsewhere.fold());
        assert_eq!(elsewhere.tz(), &Tz::UTC);
        assert_eq!(
            elsewhere.timestamp_micros() - reading.timestamp_micros(),
            2 * 3_600_000_000
        );
        // Converting to its own zone gives a zoned datetime back as it is...
        assert_eq!(reading.astimezone(&madrid).unwrap(), reading);
        assert_eq!(elsewhere.astimezone(&Tz::UTC).unwrap(), elsewhere);
        // ...and a fixed offset other than UTC through the conversion.
        let plus_one = Tz::fixed(3600).unwrap();
        let fixed = reading.with_tz(&plus_one);
        assert!(!fixed.astimezone(&plus_one).unwrap().fold());
    }

    #[test]
    fn now_is_the_system_clock() {
        let madrid = madrid();
        let before = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_micros() as i64;
        let now = DateTime::now(&madrid);
        let after = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_micros() as i64;
        assert!((before..=after).contains(&now.timestamp_micros()));
        assert_eq!(now.tz(), &madrid);
        assert_eq!(
            DateTime::now(&Tz::UTC).py_cmp(&now.checked_sub(TimeDelta::HOUR).unwrap()),
            Ordering::Greater
        );
    }
}
