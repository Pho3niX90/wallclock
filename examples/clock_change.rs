//! The repeated hour and the skipped hour in Europe/Madrid in 2026.
//!
//! On 25 October 2026 the clocks go back from 03:00 to 02:00, so 02:30 happens
//! twice. On 29 March 2026 they jump from 02:00 to 03:00, so 02:30 never
//! happens. `fold` selects which side of the change to read each time by.
//!
//! Run with: `cargo run --example clock_change`

use wallclock::{DateTime, Error, TimeDelta, Tz};

fn format_offset(delta: TimeDelta) -> String {
    let micros = delta.total_microseconds();
    let sign = if micros < 0 { '-' } else { '+' };
    let total_minutes = micros.unsigned_abs() / 60_000_000;
    format!("{sign}{:02}:{:02}", total_minutes / 60, total_minutes % 60)
}

fn show(label: &str, reading: &DateTime) -> Result<(), Error> {
    let utc = reading.astimezone(&Tz::UTC)?;
    let back = utc.astimezone(reading.tz())?;
    println!(
        "  {label:<12}  {}  {:<4}  offset {}  UTC: {}  round-trip: {}",
        reading.isoformat(),
        reading.tzname(),
        format_offset(reading.utcoffset()),
        utc.isoformat(),
        back.isoformat(),
    );
    Ok(())
}

fn main() -> Result<(), Error> {
    let madrid = Tz::zone("Europe/Madrid")?;

    println!("Repeated hour (25 October 2026, 02:30 in Europe/Madrid):");
    let first = DateTime::new(2026, 10, 25, &madrid)?.with_time(2, 30, 0, 0)?;
    let second = first.with_fold(true);
    show("fold = false", &first)?;
    show("fold = true", &second)?;
    let gap_hours = (second.timestamp_micros() - first.timestamp_micros()) / 3_600_000_000;
    println!("  Difference between the two instants: {gap_hours} hour");
    println!();

    println!("Skipped hour (29 March 2026, 02:30 in Europe/Madrid):");
    let before = DateTime::new(2026, 3, 29, &madrid)?.with_time(2, 30, 0, 0)?;
    let after = before.with_fold(true);
    show("fold = false", &before)?;
    show("fold = true", &after)?;
    Ok(())
}
