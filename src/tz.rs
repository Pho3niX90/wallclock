//! Time zones: Python's `timezone` (a fixed offset) and `zoneinfo.ZoneInfo`.

use std::fmt::Write as _;
use std::sync::Arc;

use jiff::tz::AmbiguousOffset;

use crate::civil;
use crate::error::Error;
use crate::timedelta::TimeDelta;

const MICROS_PER_SECOND: i64 = 1_000_000;
const MICROS_PER_DAY: i64 = 86_400 * MICROS_PER_SECOND;

/// A time zone: a fixed offset from UTC (Python's `datetime.timezone`) or a
/// zone of the IANA database (Python's `zoneinfo.ZoneInfo`).
///
/// Two zones are equal when they have the same offset, or the same key.
/// Python compares two datetimes by their wall clocks when they share one
/// `tzinfo` object; equal `Tz` values stand for that here.
///
/// # Examples
///
/// ```
/// use wallclock::Tz;
///
/// let madrid = Tz::zone("Europe/Madrid")?;
/// let india = Tz::fixed(5 * 3600 + 30 * 60)?;
///
/// assert_eq!(madrid, Tz::zone("Europe/Madrid")?);
/// assert_ne!(madrid, india);
/// # Ok::<(), wallclock::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Tz(Repr);

#[derive(Debug, Clone)]
enum Repr {
    /// `utcoffset()` in microseconds, strictly between -24 and 24 hours.
    Fixed(i64),
    Zone(Arc<Zone>),
}

#[derive(Debug)]
struct Zone {
    key: String,
    zone: jiff::tz::TimeZone,
}

impl PartialEq for Repr {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Fixed(a), Self::Fixed(b)) => a == b,
            (Self::Zone(a), Self::Zone(b)) => a.key == b.key,
            _ => false,
        }
    }
}

impl Eq for Repr {}

impl std::hash::Hash for Repr {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Self::Fixed(offset) => (0u8, offset).hash(state),
            Self::Zone(zone) => (1u8, &zone.key).hash(state),
        }
    }
}

/// What a zone says about one wall-clock time.
pub(crate) struct Local {
    /// `utcoffset()` in microseconds.
    pub offset: i64,
    /// Whether the wall-clock time happens twice (clocks went back over it).
    pub ambiguous: bool,
    /// The offset of the other reading, where the time is ambiguous or
    /// skipped; otherwise the same as `offset`.
    pub other: i64,
}

impl Tz {
    /// `timezone.utc`.
    ///
    /// # Examples
    ///
    /// ```
    /// use wallclock::{DateTime, Tz};
    ///
    /// let noon = DateTime::new(2026, 7, 1, &Tz::UTC)?.with_time(12, 0, 0, 0)?;
    /// assert_eq!(noon.isoformat(), "2026-07-01T12:00:00+00:00");
    /// assert_eq!(noon.tzname(), "UTC");
    /// # Ok::<(), wallclock::Error>(())
    /// ```
    pub const UTC: Self = Self(Repr::Fixed(0));

    /// `timezone(timedelta(seconds=seconds))`: a fixed offset, east of UTC
    /// when positive. Fails with [`Error::InvalidValue`] at 24 hours or more
    /// either way.
    ///
    /// # Examples
    ///
    /// ```
    /// use wallclock::{DateTime, Error, Tz};
    ///
    /// let offset = Tz::fixed(5 * 3600 + 30 * 60)?;
    /// let noon = DateTime::new(2026, 7, 1, &offset)?.with_time(12, 0, 0, 0)?;
    /// assert_eq!(noon.isoformat(), "2026-07-01T12:00:00+05:30");
    /// assert!(matches!(Tz::fixed(24 * 3600), Err(Error::InvalidValue(_))));
    /// # Ok::<(), wallclock::Error>(())
    /// ```
    pub fn fixed(seconds: i32) -> Result<Self, Error> {
        Self::fixed_delta(TimeDelta::new(0, i64::from(seconds), 0)?)
    }

    /// `timezone(offset)`: Python allows an offset down to the microsecond.
    ///
    /// # Examples
    ///
    /// ```
    /// use wallclock::{DateTime, TimeDelta, Tz};
    ///
    /// let offset = TimeDelta::new(0, 5 * 3600 + 30 * 60, 500_000)?;
    /// let tz = Tz::fixed_delta(offset)?;
    /// let noon = DateTime::new(2026, 7, 1, &tz)?.with_time(12, 0, 0, 0)?;
    /// assert_eq!(noon.isoformat(), "2026-07-01T12:00:00+05:30:00.500000");
    /// # Ok::<(), wallclock::Error>(())
    /// ```
    pub fn fixed_delta(offset: TimeDelta) -> Result<Self, Error> {
        let micros = offset.total_microseconds();
        if micros.unsigned_abs() >= MICROS_PER_DAY as u128 {
            return Err(Error::InvalidValue(
                "offset must be a timedelta strictly between -timedelta(hours=24) and timedelta(hours=24)",
            ));
        }
        Ok(Self(Repr::Fixed(micros as i64)))
    }

    /// `ZoneInfo(key)`: the zone of that name, such as `"Europe/Madrid"`,
    /// from the system's time zone database, or from the copy compiled in
    /// where the system has none (see the crate documentation).
    ///
    /// Keys are matched case-sensitively, as CPython matches them on
    /// case-sensitive databases. Fails with [`Error::InvalidValue`] when
    /// `key` is not a normalised relative path (empty, absolute, trailing or
    /// doubled slash, `.` or `..` segment, or embedded NUL) and with
    /// [`Error::UnknownZone`] when no zone has that exact name.
    ///
    /// # Examples
    ///
    /// ```
    /// use wallclock::{DateTime, Error, Tz};
    ///
    /// let madrid = Tz::zone("Europe/Madrid")?;
    /// let day = DateTime::new(2026, 7, 1, &madrid)?;
    /// assert_eq!(day.tzname(), "CEST");
    ///
    /// assert!(matches!(Tz::zone("europe/madrid"), Err(Error::UnknownZone(_))));
    /// assert!(matches!(Tz::zone("/Europe/Madrid"), Err(Error::InvalidValue(_))));
    /// # Ok::<(), wallclock::Error>(())
    /// ```
    pub fn zone(key: &str) -> Result<Self, Error> {
        validate_zone_key(key)?;
        let zone = jiff::tz::db()
            .get(key)
            .ok()
            .filter(|zone| zone.iana_name() == Some(key))
            .ok_or_else(|| Error::UnknownZone(key.to_owned()))?;
        Ok(Self(Repr::Zone(Arc::new(Zone {
            key: key.to_owned(),
            zone,
        }))))
    }

    /// `ZoneInfo.key`: the name the zone was asked for by. `None` for a
    /// fixed offset.
    ///
    /// # Examples
    ///
    /// ```
    /// use wallclock::Tz;
    ///
    /// let tokyo = Tz::zone("Asia/Tokyo")?;
    /// assert_eq!(tokyo.key(), Some("Asia/Tokyo"));
    /// assert_eq!(Tz::UTC.key(), None);
    /// # Ok::<(), wallclock::Error>(())
    /// ```
    pub fn key(&self) -> Option<&str> {
        match &self.0 {
            Repr::Fixed(_) => None,
            Repr::Zone(zone) => Some(&zone.key),
        }
    }

    /// The offset of a fixed-offset zone. `None` for a zone of the database,
    /// whose offset depends on the date.
    ///
    /// # Examples
    ///
    /// ```
    /// use wallclock::{TimeDelta, Tz};
    ///
    /// let offset = Tz::fixed(9 * 3600)?;
    /// let tokyo = Tz::zone("Asia/Tokyo")?;
    /// assert_eq!(offset.fixed_offset(), Some(TimeDelta::from_hours(9)?));
    /// assert_eq!(tokyo.fixed_offset(), None);
    /// # Ok::<(), wallclock::Error>(())
    /// ```
    pub fn fixed_offset(&self) -> Option<TimeDelta> {
        match &self.0 {
            Repr::Fixed(micros) => Some(delta(*micros)),
            Repr::Zone(_) => None,
        }
    }

    /// The offset for the wall-clock time `wall` (microseconds from
    /// 1970-01-01T00:00 on that clock), read with `fold`.
    ///
    /// PEP 495: where the time happens twice, `fold` picks the second; where
    /// it is skipped, `fold = false` reads it by the rules before the change
    /// and `fold = true` by those after.
    pub(crate) fn local(&self, wall: i64, fold: bool) -> Local {
        let zone = match &self.0 {
            Repr::Fixed(offset) => {
                return Local {
                    offset: *offset,
                    ambiguous: false,
                    other: *offset,
                };
            }
            Repr::Zone(zone) => zone,
        };
        let micros = |offset: jiff::tz::Offset| i64::from(offset.seconds()) * MICROS_PER_SECOND;
        match zone
            .zone
            .to_ambiguous_timestamp(civil::to_jiff(wall))
            .offset()
        {
            AmbiguousOffset::Unambiguous { offset } => Local {
                offset: micros(offset),
                ambiguous: false,
                other: micros(offset),
            },
            AmbiguousOffset::Gap { before, after } => {
                let (offset, other) = if fold {
                    (after, before)
                } else {
                    (before, after)
                };
                Local {
                    offset: micros(offset),
                    ambiguous: false,
                    other: micros(other),
                }
            }
            AmbiguousOffset::Fold { before, after } => {
                let (offset, other) = if fold {
                    (after, before)
                } else {
                    (before, after)
                };
                Local {
                    offset: micros(offset),
                    ambiguous: true,
                    other: micros(other),
                }
            }
        }
    }

    /// The offset at the instant `utc` (microseconds from the Unix epoch).
    pub(crate) fn offset_at(&self, utc: i64) -> i64 {
        match &self.0 {
            Repr::Fixed(offset) => *offset,
            Repr::Zone(zone) => {
                i64::from(zone.zone.to_offset(civil::timestamp(utc)).seconds()) * MICROS_PER_SECOND
            }
        }
    }

    /// `tzname()` for the wall-clock time `wall`, read with `fold`.
    pub(crate) fn name(&self, wall: i64, fold: bool) -> String {
        let zone = match &self.0 {
            Repr::Fixed(0) => return "UTC".to_owned(),
            Repr::Fixed(offset) => return format!("UTC{}", offset_text(*offset)),
            Repr::Zone(zone) => zone,
        };
        let local = self.local(wall, fold);
        // The period the reading belongs to. A skipped time read by the
        // rules before the change lies, as an instant, after it: the other
        // offset gives an instant on the right side.
        let skipped = !local.ambiguous && local.other != local.offset;
        let instant = wall - if skipped { local.other } else { local.offset };
        zone.zone
            .to_offset_info(civil::timestamp(instant))
            .abbreviation()
            .to_owned()
    }
}

fn validate_zone_key(key: &str) -> Result<(), Error> {
    if key.contains('\0') {
        return Err(Error::InvalidValue("embedded null character"));
    }
    if key.starts_with('/') {
        return Err(Error::InvalidValue(
            "ZoneInfo keys may not be absolute paths",
        ));
    }
    if key.is_empty()
        || key
            .split('/')
            .any(|segment| matches!(segment, "" | "." | ".."))
    {
        return Err(Error::InvalidValue(
            "ZoneInfo keys must be normalized relative paths",
        ));
    }
    Ok(())
}

fn delta(micros: i64) -> TimeDelta {
    TimeDelta::from_micros(i128::from(micros)).expect("an offset is under a day")
}

/// An offset as `isoformat()` writes it: `+HH:MM`, with `:SS` and
/// `.ffffff` only where they are not zero.
pub(crate) fn offset_text(micros: i64) -> String {
    let sign = if micros < 0 { '-' } else { '+' };
    let size = micros.unsigned_abs();
    let seconds = size / MICROS_PER_SECOND as u64;
    let mut text = format!("{sign}{:02}:{:02}", seconds / 3600, seconds / 60 % 60);
    let (second, micro) = (seconds % 60, size % MICROS_PER_SECOND as u64);
    if second != 0 || micro != 0 {
        let _ = write!(text, ":{second:02}");
    }
    if micro != 0 {
        let _ = write!(text, ".{micro:06}");
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_offsets_are_under_a_day() {
        assert_eq!(Tz::fixed(0).unwrap(), Tz::UTC);
        assert_eq!(Tz::fixed(86_399).unwrap().name(0, false), "UTC+23:59:59");
        assert_eq!(Tz::fixed(-19_800).unwrap().name(0, false), "UTC-05:30");
        assert_eq!(Tz::UTC.name(0, true), "UTC");
        assert_eq!(
            Tz::fixed(86_400).unwrap_err().python_exception(),
            "ValueError"
        );
        assert!(Tz::fixed(-86_400).is_err());
        let tiny = Tz::fixed_delta(TimeDelta::MICROSECOND).unwrap();
        assert_eq!(tiny.name(0, false), "UTC+00:00:00.000001");
        assert_eq!(tiny.fixed_offset(), Some(TimeDelta::MICROSECOND));
        assert_eq!(tiny.key(), None);
        assert!(Tz::fixed_delta(TimeDelta::MAX).is_err());
    }

    #[test]
    fn zones_come_from_the_database() {
        let madrid = Tz::zone("Europe/Madrid").unwrap();
        assert_eq!(madrid.key(), Some("Europe/Madrid"));
        assert_eq!(madrid.fixed_offset(), None);
        assert_eq!(madrid, Tz::zone("Europe/Madrid").unwrap());
        assert_ne!(madrid, Tz::zone("Europe/Paris").unwrap());
        assert_ne!(madrid, Tz::fixed(3600).unwrap());
        let error = Tz::zone("Mars/Olympus_Mons").unwrap_err();
        assert_eq!(error.python_exception(), "ZoneInfoNotFoundError");
        assert!(error.to_string().contains("Mars/Olympus_Mons"));
        for wrong_case in ["europe/madrid", "EUROPE/MADRID", "us/eastern", "utc"] {
            assert_eq!(
                Tz::zone(wrong_case).unwrap_err().python_exception(),
                "ZoneInfoNotFoundError",
                "{wrong_case}"
            );
        }
        for invalid in [
            "",
            "/",
            "/Europe/Madrid",
            "Europe/Madrid/",
            "Europe//Madrid",
            ".",
            "..",
            "./UTC",
            "UTC/.",
            "../UTC",
            "UTC/..",
            "Europe/../Europe/Madrid",
            "Europe/Madrid\0",
        ] {
            assert_eq!(
                Tz::zone(invalid).unwrap_err().python_exception(),
                "ValueError",
                "{invalid:?}"
            );
        }
        // A link keeps the name it was asked for by. (Older links such as
        // `US/Eastern` are an optional package on some systems, so they are
        // not assumed here.)
        assert_eq!(Tz::zone("Etc/UTC").unwrap().key(), Some("Etc/UTC"));

        use std::collections::HashSet;
        let set: HashSet<Tz> = [madrid.clone(), madrid, Tz::UTC, Tz::fixed(0).unwrap()]
            .into_iter()
            .collect();
        assert_eq!(set.len(), 2);
    }
}
