//! Python's `timedelta`.

use std::fmt;

use crate::error::Error;

const MICROS_PER_SECOND: i128 = 1_000_000;
const MICROS_PER_DAY: i128 = 86_400 * MICROS_PER_SECOND;

/// A duration with microsecond resolution: Python's `datetime.timedelta`.
///
/// It holds what Python's holds, from `-999999999` days up to
/// `999999999` days, 23:59:59.999999. Like Python's it is one number of
/// microseconds, shown as days (which may be negative), seconds (0 to
/// 86399) and microseconds (0 to 999999).
///
/// # Examples
///
/// ```
/// use wallclock::TimeDelta;
///
/// let delta = TimeDelta::new(1, 3600, 0)?;
/// assert_eq!(delta.to_string(), "1 day, 1:00:00");
/// assert_eq!(delta.total_seconds(), 90_000.0);
/// # Ok::<(), wallclock::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct TimeDelta {
    micros: i128,
}

impl TimeDelta {
    /// `timedelta(0)`.
    ///
    /// # Examples
    ///
    /// ```
    /// use wallclock::TimeDelta;
    ///
    /// assert_eq!(TimeDelta::ZERO.total_microseconds(), 0);
    /// assert_eq!(TimeDelta::MICROSECOND.microseconds(), 1);
    /// assert_eq!(TimeDelta::SECOND.total_microseconds(), 1_000_000);
    /// assert_eq!(TimeDelta::MINUTE.seconds(), 60);
    /// assert_eq!(TimeDelta::HOUR.seconds(), 3600);
    /// assert_eq!(TimeDelta::DAY.days(), 1);
    /// ```
    pub const ZERO: Self = Self { micros: 0 };
    /// `timedelta(microseconds=1)`, Python's `timedelta.resolution`.
    /// See [`TimeDelta::ZERO`] for an example.
    pub const MICROSECOND: Self = Self { micros: 1 };
    /// `timedelta(seconds=1)`. See [`TimeDelta::ZERO`] for an example.
    pub const SECOND: Self = Self {
        micros: MICROS_PER_SECOND,
    };
    /// `timedelta(minutes=1)`. See [`TimeDelta::ZERO`] for an example.
    pub const MINUTE: Self = Self {
        micros: 60 * MICROS_PER_SECOND,
    };
    /// `timedelta(hours=1)`. See [`TimeDelta::ZERO`] for an example.
    pub const HOUR: Self = Self {
        micros: 3600 * MICROS_PER_SECOND,
    };
    /// `timedelta(days=1)`. See [`TimeDelta::ZERO`] for an example.
    pub const DAY: Self = Self {
        micros: MICROS_PER_DAY,
    };
    /// `timedelta.min`: -999999999 days.
    ///
    /// # Examples
    ///
    /// ```
    /// use wallclock::{Error, TimeDelta};
    ///
    /// assert_eq!(TimeDelta::MIN.days(), -999_999_999);
    /// assert_eq!(TimeDelta::MAX.days(), 999_999_999);
    /// assert_eq!(TimeDelta::MAX.seconds(), 86_399);
    /// assert_eq!(TimeDelta::MAX.microseconds(), 999_999);
    /// assert_eq!(TimeDelta::MAX.checked_add(TimeDelta::MICROSECOND), Err(Error::Overflow));
    /// ```
    pub const MIN: Self = Self {
        micros: -999_999_999 * MICROS_PER_DAY,
    };
    /// `timedelta.max`: 999999999 days, 23:59:59.999999.
    /// See [`TimeDelta::MIN`] for an example.
    pub const MAX: Self = Self {
        micros: 1_000_000_000 * MICROS_PER_DAY - 1,
    };

    /// `timedelta(days=days, seconds=seconds, microseconds=microseconds)`.
    /// The arguments may have any sign and size; they are added up.
    ///
    /// Fails with [`Error::Overflow`] beyond [`TimeDelta::MIN`] and
    /// [`TimeDelta::MAX`].
    ///
    /// # Examples
    ///
    /// ```
    /// use wallclock::TimeDelta;
    ///
    /// let delta = TimeDelta::new(0, -1, 0)?;
    /// assert_eq!((delta.days(), delta.seconds(), delta.microseconds()), (-1, 86_399, 0));
    ///
    /// let carry = TimeDelta::new(1, 86_400, 1_500_000)?;
    /// assert_eq!((carry.days(), carry.seconds(), carry.microseconds()), (2, 1, 500_000));
    /// # Ok::<(), wallclock::Error>(())
    /// ```
    pub fn new(days: i64, seconds: i64, microseconds: i64) -> Result<Self, Error> {
        Self::from_micros(
            i128::from(days) * MICROS_PER_DAY
                + i128::from(seconds) * MICROS_PER_SECOND
                + i128::from(microseconds),
        )
    }

    /// A whole number of days: `timedelta(days=days)`.
    ///
    /// # Examples
    ///
    /// ```
    /// use wallclock::TimeDelta;
    ///
    /// let week = TimeDelta::from_days(7)?;
    /// assert_eq!(week.days(), 7);
    /// assert_eq!(week.total_seconds(), 604_800.0);
    /// # Ok::<(), wallclock::Error>(())
    /// ```
    pub fn from_days(days: i64) -> Result<Self, Error> {
        Self::new(days, 0, 0)
    }

    /// A whole number of hours: `timedelta(hours=hours)`.
    ///
    /// # Examples
    ///
    /// ```
    /// use wallclock::TimeDelta;
    ///
    /// let day = TimeDelta::from_hours(25)?;
    /// assert_eq!(day.days(), 1);
    /// assert_eq!(day.seconds(), 3600);
    /// # Ok::<(), wallclock::Error>(())
    /// ```
    pub fn from_hours(hours: i64) -> Result<Self, Error> {
        Self::from_micros(i128::from(hours) * 3_600 * MICROS_PER_SECOND)
    }

    /// A whole number of minutes: `timedelta(minutes=minutes)`.
    ///
    /// # Examples
    ///
    /// ```
    /// use wallclock::TimeDelta;
    ///
    /// let delta = TimeDelta::from_minutes(90)?;
    /// assert_eq!(delta.to_string(), "1:30:00");
    /// # Ok::<(), wallclock::Error>(())
    /// ```
    pub fn from_minutes(minutes: i64) -> Result<Self, Error> {
        Self::from_micros(i128::from(minutes) * 60 * MICROS_PER_SECOND)
    }

    /// A whole number of seconds: `timedelta(seconds=seconds)`.
    ///
    /// # Examples
    ///
    /// ```
    /// use wallclock::TimeDelta;
    ///
    /// let delta = TimeDelta::from_seconds(90)?;
    /// assert_eq!(delta.seconds(), 90);
    /// assert_eq!(delta.total_seconds(), 90.0);
    /// # Ok::<(), wallclock::Error>(())
    /// ```
    pub fn from_seconds(seconds: i64) -> Result<Self, Error> {
        Self::from_micros(i128::from(seconds) * MICROS_PER_SECOND)
    }

    /// A whole number of microseconds: `timedelta(microseconds=micros)`.
    ///
    /// # Examples
    ///
    /// ```
    /// use wallclock::TimeDelta;
    ///
    /// let delta = TimeDelta::from_micros(2_500_000)?;
    /// assert_eq!(delta.seconds(), 2);
    /// assert_eq!(delta.microseconds(), 500_000);
    /// # Ok::<(), wallclock::Error>(())
    /// ```
    pub fn from_micros(micros: i128) -> Result<Self, Error> {
        if (Self::MIN.micros..=Self::MAX.micros).contains(&micros) {
            Ok(Self { micros })
        } else {
            Err(Error::Overflow)
        }
    }

    /// `.days`: whole days, rounded down, so negative for a negative delta.
    ///
    /// # Examples
    ///
    /// ```
    /// use wallclock::TimeDelta;
    ///
    /// assert_eq!(TimeDelta::new(2, 3600, 0)?.days(), 2);
    /// assert_eq!(TimeDelta::new(0, -1, 0)?.days(), -1);
    /// # Ok::<(), wallclock::Error>(())
    /// ```
    pub fn days(&self) -> i64 {
        self.micros.div_euclid(MICROS_PER_DAY) as i64
    }

    /// `.seconds`: 0 to 86399, the seconds past [`TimeDelta::days`].
    ///
    /// # Examples
    ///
    /// ```
    /// use wallclock::TimeDelta;
    ///
    /// assert_eq!(TimeDelta::new(1, 90, 0)?.seconds(), 90);
    /// assert_eq!(TimeDelta::new(0, -1, 0)?.seconds(), 86_399);
    /// # Ok::<(), wallclock::Error>(())
    /// ```
    pub fn seconds(&self) -> i64 {
        (self.micros.rem_euclid(MICROS_PER_DAY) / MICROS_PER_SECOND) as i64
    }

    /// `.microseconds`: 0 to 999999.
    ///
    /// # Examples
    ///
    /// ```
    /// use wallclock::TimeDelta;
    ///
    /// assert_eq!(TimeDelta::new(0, 0, 250)?.microseconds(), 250);
    /// assert_eq!(TimeDelta::new(0, 0, -1)?.microseconds(), 999_999);
    /// # Ok::<(), wallclock::Error>(())
    /// ```
    pub fn microseconds(&self) -> i64 {
        self.micros.rem_euclid(MICROS_PER_SECOND) as i64
    }

    /// The whole delta in microseconds: `delta // timedelta(microseconds=1)`.
    ///
    /// # Examples
    ///
    /// ```
    /// use wallclock::TimeDelta;
    ///
    /// let delta = TimeDelta::new(0, 1, 500_000)?;
    /// assert_eq!(delta.total_microseconds(), 1_500_000);
    /// assert_eq!(delta.checked_neg()?.total_microseconds(), -1_500_000);
    /// # Ok::<(), wallclock::Error>(())
    /// ```
    pub fn total_microseconds(&self) -> i128 {
        self.micros
    }

    /// `.total_seconds()`: the delta in seconds as the nearest `f64`, which
    /// is what Python's division of two integers gives.
    ///
    /// # Examples
    ///
    /// ```
    /// use wallclock::TimeDelta;
    ///
    /// let delta = TimeDelta::new(0, 90, 500_000)?;
    /// assert_eq!(delta.total_seconds(), 90.5);
    /// assert_eq!(delta.checked_neg()?.total_seconds(), -90.5);
    /// # Ok::<(), wallclock::Error>(())
    /// ```
    pub fn total_seconds(&self) -> f64 {
        divide(self.micros, MICROS_PER_SECOND as u128)
    }

    /// `self + other`, or [`Error::Overflow`].
    ///
    /// # Examples
    ///
    /// ```
    /// use wallclock::{Error, TimeDelta};
    ///
    /// let sum = TimeDelta::HOUR.checked_add(TimeDelta::from_minutes(30)?)?;
    /// assert_eq!(sum.total_seconds(), 5400.0);
    /// assert_eq!(TimeDelta::MAX.checked_add(TimeDelta::MICROSECOND), Err(Error::Overflow));
    /// # Ok::<(), wallclock::Error>(())
    /// ```
    pub fn checked_add(self, other: Self) -> Result<Self, Error> {
        Self::from_micros(self.micros + other.micros)
    }

    /// `self - other`, or [`Error::Overflow`].
    ///
    /// # Examples
    ///
    /// ```
    /// use wallclock::{Error, TimeDelta};
    ///
    /// let diff = TimeDelta::HOUR.checked_sub(TimeDelta::DAY)?;
    /// assert_eq!(diff.to_string(), "-1 day, 1:00:00");
    /// assert_eq!(TimeDelta::MIN.checked_sub(TimeDelta::MICROSECOND), Err(Error::Overflow));
    /// # Ok::<(), wallclock::Error>(())
    /// ```
    pub fn checked_sub(self, other: Self) -> Result<Self, Error> {
        Self::from_micros(self.micros - other.micros)
    }

    /// `self * times`, or [`Error::Overflow`].
    ///
    /// # Examples
    ///
    /// ```
    /// use wallclock::{Error, TimeDelta};
    ///
    /// assert_eq!(TimeDelta::HOUR.checked_mul(24)?, TimeDelta::DAY);
    /// assert_eq!(TimeDelta::DAY.checked_mul(i64::MAX), Err(Error::Overflow));
    /// # Ok::<(), wallclock::Error>(())
    /// ```
    pub fn checked_mul(self, times: i64) -> Result<Self, Error> {
        self.micros
            .checked_mul(i128::from(times))
            .ok_or(Error::Overflow)
            .and_then(Self::from_micros)
    }

    /// `-self`, or [`Error::Overflow`]: `timedelta.max` has no negative.
    ///
    /// # Examples
    ///
    /// ```
    /// use wallclock::{Error, TimeDelta};
    ///
    /// assert_eq!(TimeDelta::HOUR.checked_neg()?.total_seconds(), -3600.0);
    /// assert_eq!(TimeDelta::MAX.checked_neg(), Err(Error::Overflow));
    /// # Ok::<(), wallclock::Error>(())
    /// ```
    pub fn checked_neg(self) -> Result<Self, Error> {
        Self::from_micros(-self.micros)
    }
}

/// `numerator / denominator` as the nearest `f64`, ties to even: Python's
/// true division of two integers.
pub(crate) fn divide(numerator: i128, denominator: u128) -> f64 {
    if numerator == 0 {
        return 0.0;
    }
    let size = numerator.unsigned_abs();
    // Scale so the quotient has at least 64 bits, eleven more than an `f64`
    // holds, and fold the remainder into the lowest: the conversion to
    // `f64` then rounds exactly once.
    let bits = |value: u128| 128 - value.leading_zeros() as i32;
    let shift = (64 + bits(denominator) - bits(size)).max(0);
    let scaled = size << shift;
    let mut quotient = scaled / denominator;
    if scaled % denominator != 0 {
        quotient |= 1;
    }
    let value = quotient as f64 / 2f64.powi(shift);
    if numerator < 0 { -value } else { value }
}

/// Python's `str(timedelta)`: `1 day, 2:03:04.000005`, `-1 day, 23:59:59`.
///
/// # Examples
///
/// ```
/// use wallclock::TimeDelta;
///
/// assert_eq!(TimeDelta::new(1, 7384, 5)?.to_string(), "1 day, 2:03:04.000005");
/// assert_eq!(TimeDelta::new(0, -1, 0)?.to_string(), "-1 day, 23:59:59");
/// # Ok::<(), wallclock::Error>(())
/// ```
impl fmt::Display for TimeDelta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let days = self.days();
        if days != 0 {
            let plural = if days.abs() == 1 { "" } else { "s" };
            write!(f, "{days} day{plural}, ")?;
        }
        let seconds = self.seconds();
        write!(
            f,
            "{}:{:02}:{:02}",
            seconds / 3600,
            seconds / 60 % 60,
            seconds % 60
        )?;
        match self.microseconds() {
            0 => Ok(()),
            micros => write!(f, ".{micros:06}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fields_are_normalised_as_python_does() {
        let delta = TimeDelta::new(0, -1, 0).unwrap();
        assert_eq!(
            (delta.days(), delta.seconds(), delta.microseconds()),
            (-1, 86_399, 0)
        );
        let delta = TimeDelta::new(1, 86_400 + 3661, 2_000_005).unwrap();
        assert_eq!(
            (delta.days(), delta.seconds(), delta.microseconds()),
            (2, 3663, 5)
        );
        assert_eq!(delta.total_microseconds(), 176_463_000_005);
        assert_eq!(TimeDelta::default(), TimeDelta::ZERO);
        assert_eq!(TimeDelta::from_days(1).unwrap(), TimeDelta::DAY);
        assert!(TimeDelta::HOUR > TimeDelta::MINUTE);
    }

    #[test]
    fn the_range_is_pythons() {
        assert_eq!(TimeDelta::new(-999_999_999, 0, 0).unwrap(), TimeDelta::MIN);
        assert_eq!(
            TimeDelta::new(999_999_999, 86_399, 999_999).unwrap(),
            TimeDelta::MAX
        );
        assert_eq!(TimeDelta::new(1_000_000_000, 0, 0), Err(Error::Overflow));
        assert_eq!(TimeDelta::new(-999_999_999, 0, -1), Err(Error::Overflow));
        assert_eq!(
            TimeDelta::MAX.checked_add(TimeDelta::MICROSECOND),
            Err(Error::Overflow)
        );
        assert_eq!(
            TimeDelta::MIN.checked_sub(TimeDelta::MICROSECOND),
            Err(Error::Overflow)
        );
        assert_eq!(TimeDelta::DAY.checked_mul(i64::MAX), Err(Error::Overflow));
        assert_eq!(TimeDelta::MAX.checked_mul(i64::MAX), Err(Error::Overflow));
        assert_eq!(TimeDelta::MAX.checked_neg(), Err(Error::Overflow));
        assert_eq!(
            TimeDelta::MIN.checked_neg().unwrap(),
            TimeDelta::from_days(999_999_999).unwrap()
        );
        assert_eq!(
            TimeDelta::DAY.checked_neg().unwrap(),
            TimeDelta::new(-1, 0, 0).unwrap()
        );
        assert_eq!(
            TimeDelta::HOUR.checked_mul(24).unwrap(),
            TimeDelta::SECOND.checked_mul(86_400).unwrap()
        );

        let max_hours = 1_000_000_000 * 24 - 1;
        let min_hours = -999_999_999 * 24;
        assert_eq!(
            TimeDelta::from_hours(max_hours).unwrap(),
            TimeDelta::new(999_999_999, 23 * 3_600, 0).unwrap()
        );
        assert_eq!(TimeDelta::from_hours(max_hours + 1), Err(Error::Overflow));
        assert_eq!(TimeDelta::from_hours(min_hours).unwrap(), TimeDelta::MIN);
        assert_eq!(TimeDelta::from_hours(min_hours - 1), Err(Error::Overflow));
        assert_eq!(TimeDelta::from_hours(i64::MAX), Err(Error::Overflow));
        assert_eq!(TimeDelta::from_hours(i64::MIN), Err(Error::Overflow));

        let max_minutes = 1_000_000_000 * 1_440 - 1;
        let min_minutes = -999_999_999 * 1_440;
        assert_eq!(
            TimeDelta::from_minutes(max_minutes).unwrap(),
            TimeDelta::new(999_999_999, 23 * 3_600 + 59 * 60, 0).unwrap()
        );
        assert_eq!(
            TimeDelta::from_minutes(max_minutes + 1),
            Err(Error::Overflow)
        );
        assert_eq!(
            TimeDelta::from_minutes(min_minutes).unwrap(),
            TimeDelta::MIN
        );
        assert_eq!(
            TimeDelta::from_minutes(min_minutes - 1),
            Err(Error::Overflow)
        );
        assert_eq!(TimeDelta::from_minutes(i64::MAX), Err(Error::Overflow));
        assert_eq!(TimeDelta::from_minutes(i64::MIN), Err(Error::Overflow));

        let max_seconds = 1_000_000_000 * 86_400 - 1;
        let min_seconds = -999_999_999 * 86_400;
        assert_eq!(
            TimeDelta::from_seconds(max_seconds).unwrap(),
            TimeDelta::new(999_999_999, 86_399, 0).unwrap()
        );
        assert_eq!(
            TimeDelta::from_seconds(max_seconds + 1),
            Err(Error::Overflow)
        );
        assert_eq!(
            TimeDelta::from_seconds(min_seconds).unwrap(),
            TimeDelta::MIN
        );
        assert_eq!(
            TimeDelta::from_seconds(min_seconds - 1),
            Err(Error::Overflow)
        );
        assert_eq!(TimeDelta::from_seconds(i64::MAX), Err(Error::Overflow));
        assert_eq!(TimeDelta::from_seconds(i64::MIN), Err(Error::Overflow));
    }

    #[test]
    fn total_seconds_rounds_once() {
        assert_eq!(TimeDelta::ZERO.total_seconds(), 0.0);
        assert_eq!(TimeDelta::new(0, 1, 500_000).unwrap().total_seconds(), 1.5);
        assert_eq!(
            TimeDelta::new(0, -1, -500_000).unwrap().total_seconds(),
            -1.5
        );
        // Python: timedelta(microseconds=253402300799999999).total_seconds()
        assert_eq!(
            TimeDelta::from_micros(253_402_300_799_999_999)
                .unwrap()
                .total_seconds(),
            253_402_300_800.0
        );
        // timedelta.max.total_seconds() and timedelta.min.total_seconds()
        assert_eq!(TimeDelta::MAX.total_seconds(), 86_400_000_000_000.0);
        assert_eq!(TimeDelta::MIN.total_seconds(), -86_399_999_913_600.0);
        assert_eq!(TimeDelta::MICROSECOND.total_seconds(), 1e-6);
    }

    #[test]
    fn text_is_pythons() {
        let text =
            |days, seconds, micros| TimeDelta::new(days, seconds, micros).unwrap().to_string();
        assert_eq!(text(0, 0, 0), "0:00:00");
        assert_eq!(text(1, 7384, 5), "1 day, 2:03:04.000005");
        assert_eq!(text(0, -1, 0), "-1 day, 23:59:59");
        assert_eq!(text(-2, 0, 0), "-2 days, 0:00:00");
        assert_eq!(text(400, 86_399, 0), "400 days, 23:59:59");
    }
}
