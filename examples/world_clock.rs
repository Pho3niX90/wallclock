//! One instant shown in five time zones, with each zone's abbreviation and
//! offset from UTC.
//!
//! Run with: `cargo run --example world_clock`

use wallclock::{DateTime, Error, TimeDelta, Tz};

fn format_offset(delta: TimeDelta) -> String {
    let micros = delta.total_microseconds();
    let sign = if micros < 0 { '-' } else { '+' };
    let total_minutes = micros.unsigned_abs() / 60_000_000;
    format!("{sign}{:02}:{:02}", total_minutes / 60, total_minutes % 60)
}

fn main() -> Result<(), Error> {
    let instant = DateTime::new(2026, 7, 1, &Tz::UTC)?.with_time(12, 0, 0, 0)?;
    let zones = [
        ("UTC", Tz::UTC),
        ("America/New_York", Tz::zone("America/New_York")?),
        ("Europe/Madrid", Tz::zone("Europe/Madrid")?),
        ("Asia/Kolkata", Tz::zone("Asia/Kolkata")?),
        ("Asia/Tokyo", Tz::zone("Asia/Tokyo")?),
    ];

    println!("Instant {} around the world:", instant.isoformat());
    for (label, tz) in zones {
        let local = instant.astimezone(&tz)?;
        println!(
            "  {label:<18}  {}  {:<4}  UTC{}",
            local.isoformat(),
            local.tzname(),
            format_offset(local.utcoffset()),
        );
    }
    Ok(())
}
