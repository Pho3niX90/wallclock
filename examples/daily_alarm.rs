//! "Every day at 08:00 in Madrid" across the autumn clock change of
//! 25 October 2026. The wall-clock time stays at 08:00 while the UTC instant
//! shifts and the day of the change lasts 25 hours.
//!
//! Run with: `cargo run --example daily_alarm`

use wallclock::{DateTime, Error, TimeDelta, Tz};

const DAYS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

fn main() -> Result<(), Error> {
    let madrid = Tz::zone("Europe/Madrid")?;
    let mut alarm = DateTime::new(2026, 10, 22, &madrid)?.with_time(8, 0, 0, 0)?;

    println!("Daily alarm at 08:00 in Europe/Madrid (week of 25 October 2026):");
    let mut previous: Option<DateTime> = None;
    for _ in 0..7 {
        let day = DAYS[alarm.weekday() as usize];
        let utc = alarm.astimezone(&Tz::UTC)?;
        let elapsed = match &previous {
            Some(prev) => {
                let hours = (alarm.timestamp_micros() - prev.timestamp_micros()) / 3_600_000_000;
                format!("  (+{hours}h)")
            }
            None => String::new(),
        };
        println!(
            "  {day}  {}  {:<4}  UTC: {}{elapsed}",
            alarm.isoformat(),
            alarm.tzname(),
            utc.isoformat(),
        );
        let next = alarm.checked_add(TimeDelta::DAY)?;
        previous = Some(alarm);
        alarm = next;
    }
    Ok(())
}
