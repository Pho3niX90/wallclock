//! The proleptic Gregorian calendar on a wall clock, as microseconds from
//! 1970-01-01T00:00:00.

const MICROS_PER_SECOND: i64 = 1_000_000;
const MICROS_PER_DAY: i64 = 86_400 * MICROS_PER_SECOND;

/// 0001-01-01T00:00:00, `datetime.min`.
pub(crate) const MIN: i64 = -62_135_596_800 * MICROS_PER_SECOND;
/// 9999-12-31T23:59:59.999999, `datetime.max`.
pub(crate) const MAX: i64 = 253_402_300_800 * MICROS_PER_SECOND - 1;

/// A date and a time of day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Fields {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
    pub microsecond: u32,
}

/// Days from 1970-01-01 to a date (Howard Hinnant's `days_from_civil`).
fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let year = i64::from(year) - i64::from(month <= 2);
    let era = year.div_euclid(400);
    let year_of_era = year.rem_euclid(400);
    let month = i64::from(month);
    let day_of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + i64::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// The date `days` after 1970-01-01 (`civil_from_days`).
fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let days = days + 719_468;
    let era = days.div_euclid(146_097);
    let day_of_era = days.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year as i32, month as u32, day as u32)
}

/// The number of days in a month.
pub(crate) fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        4 | 6 | 9 | 11 => 30,
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        _ => 31,
    }
}

/// The wall-clock microseconds of `fields`, which must be a valid date and time.
pub(crate) fn from_fields(fields: Fields) -> i64 {
    let seconds =
        i64::from(fields.hour) * 3600 + i64::from(fields.minute) * 60 + i64::from(fields.second);
    days_from_civil(fields.year, fields.month, fields.day) * MICROS_PER_DAY
        + seconds * MICROS_PER_SECOND
        + i64::from(fields.microsecond)
}

/// The date and time of day at `wall`.
pub(crate) fn to_fields(wall: i64) -> Fields {
    let (year, month, day) = civil_from_days(wall.div_euclid(MICROS_PER_DAY));
    let of_day = wall.rem_euclid(MICROS_PER_DAY);
    let seconds = of_day / MICROS_PER_SECOND;
    Fields {
        year,
        month,
        day,
        hour: (seconds / 3600) as u32,
        minute: (seconds / 60 % 60) as u32,
        second: (seconds % 60) as u32,
        microsecond: (of_day % MICROS_PER_SECOND) as u32,
    }
}

/// Monday is 0: `date.weekday()`.
pub(crate) fn weekday(wall: i64) -> u32 {
    // 1970-01-01 was a Thursday.
    (wall.div_euclid(MICROS_PER_DAY) + 3).rem_euclid(7) as u32
}

/// `wall` (within [`MIN`] to [`MAX`]) as the civil datetime jiff looks zones up by.
pub(crate) fn to_jiff(wall: i64) -> jiff::civil::DateTime {
    let fields = to_fields(wall);
    jiff::civil::DateTime::new(
        fields.year as i16,
        fields.month as i8,
        fields.day as i8,
        fields.hour as i8,
        fields.minute as i8,
        fields.second as i8,
        (fields.microsecond * 1000) as i32,
    )
    .expect("years 1 to 9999 are civil datetimes")
}

/// The instant `utc` microseconds from the Unix epoch, clamped to what jiff
/// holds (it stops a day or two short of the end of year 9999). A
/// wall-clock time at either end of the calendar can be an instant just
/// outside; a zone's rules are the same there.
pub(crate) fn timestamp(utc: i64) -> jiff::Timestamp {
    // jiff's last instant has a fraction of a second its own constructor
    // refuses; whole seconds are enough here.
    let utc = utc.clamp(
        MIN,
        MAX.min(jiff::Timestamp::MAX.as_second() * MICROS_PER_SECOND),
    );
    jiff::Timestamp::from_microsecond(utc).expect("clamped to jiff's range")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fields(
        year: i32,
        month: u32,
        day: u32,
        hour: u32,
        minute: u32,
        second: u32,
        microsecond: u32,
    ) -> Fields {
        Fields {
            year,
            month,
            day,
            hour,
            minute,
            second,
            microsecond,
        }
    }

    #[test]
    fn the_ends_of_the_calendar() {
        assert_eq!(from_fields(fields(1970, 1, 1, 0, 0, 0, 0)), 0);
        assert_eq!(from_fields(fields(1, 1, 1, 0, 0, 0, 0)), MIN);
        assert_eq!(from_fields(fields(9999, 12, 31, 23, 59, 59, 999_999)), MAX);
        assert_eq!(to_fields(MIN), fields(1, 1, 1, 0, 0, 0, 0));
        assert_eq!(to_fields(MAX), fields(9999, 12, 31, 23, 59, 59, 999_999));
        assert_eq!(to_fields(-1), fields(1969, 12, 31, 23, 59, 59, 999_999));
    }

    #[test]
    fn every_day_round_trips() {
        let mut wall = MIN;
        let mut previous = to_fields(wall);
        while wall <= MAX {
            let now = to_fields(wall);
            assert_eq!(from_fields(now), wall);
            assert!(now.day <= days_in_month(now.year, now.month));
            if wall != MIN {
                let next_day = (previous.day % days_in_month(previous.year, previous.month)) + 1;
                assert_eq!(now.day, next_day, "{previous:?} then {now:?}");
            }
            previous = now;
            wall += MICROS_PER_DAY;
        }
    }

    #[test]
    fn weekdays_and_month_lengths() {
        assert_eq!(weekday(0), 3); // Thursday
        assert_eq!(weekday(from_fields(fields(2026, 11, 21, 12, 0, 0, 0))), 5); // Saturday
        assert_eq!(weekday(MIN), 0); // Monday, 1 January of year 1
        assert_eq!(weekday(MAX), 4); // Friday
        assert_eq!(days_in_month(2024, 2), 29);
        assert_eq!(days_in_month(1900, 2), 28);
        assert_eq!(days_in_month(2000, 2), 29);
        assert_eq!(days_in_month(2026, 11), 30);
        assert_eq!(days_in_month(2026, 12), 31);
    }

    #[test]
    fn jiff_values() {
        assert_eq!(to_jiff(MAX).to_string(), "9999-12-31T23:59:59.999999");
        assert_eq!(timestamp(MAX + 5), timestamp(MAX));
        assert_eq!(timestamp(MIN - 5), timestamp(MIN));
        assert_eq!(timestamp(i64::MAX), timestamp(MAX));
        assert_eq!(timestamp(i64::MIN), timestamp(MIN));
        assert_eq!(timestamp(MIN).as_microsecond(), MIN);
        assert!(timestamp(MAX).as_microsecond() > MAX - 2 * MICROS_PER_DAY);
        assert_eq!(timestamp(1_500_000).as_microsecond(), 1_500_000);
    }
}
