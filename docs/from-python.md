# Coming from Python

wallclock follows Python's `datetime` and `zoneinfo` rule for rule, so a line of Python translates almost word for word. This page gives the translation and the few places where Rust makes it look different.

## The same thing in both

Python:

```python
from datetime import datetime, timedelta, timezone
from zoneinfo import ZoneInfo

madrid = ZoneInfo("Europe/Madrid")
at = datetime(2026, 10, 24, 8, 0, tzinfo=madrid)
next_day = at + timedelta(days=1)

print(next_day.isoformat())            # 2026-10-25T08:00:00+01:00
print((next_day - at).days)            # 1
print(next_day.astimezone(timezone.utc).isoformat())   # 2026-10-25T07:00:00+00:00
```

Rust:

```rust
use wallclock::{DateTime, TimeDelta, Tz};

fn main() -> Result<(), wallclock::Error> {
    let madrid = Tz::zone("Europe/Madrid")?;
    let at = DateTime::new(2026, 10, 24, &madrid)?.with_time(8, 0, 0, 0)?;
    let next_day = at.checked_add(TimeDelta::DAY)?;

    assert_eq!(next_day.isoformat(), "2026-10-25T08:00:00+01:00");
    assert_eq!(next_day.since(&at).days(), 1);
    assert_eq!(next_day.astimezone(&Tz::UTC)?.isoformat(), "2026-10-25T07:00:00+00:00");
    Ok(())
}
```

## Translation table

### Zones

| Python | wallclock |
|---|---|
| `ZoneInfo("Europe/Madrid")` | `Tz::zone("Europe/Madrid")?` |
| `timezone.utc` | `Tz::UTC` |
| `timezone(timedelta(hours=2))` | `Tz::fixed(2 * 3600)?` |
| `timezone(some_timedelta)` | `Tz::fixed_delta(some_timedelta)?` |
| `zone.key` | `tz.key()` (`None` for a fixed offset) |

### Making a datetime

| Python | wallclock |
|---|---|
| `datetime(2026, 10, 24, tzinfo=tz)` | `DateTime::new(2026, 10, 24, &tz)?` |
| `datetime(2026, 10, 24, 8, 30, 15, 250, tzinfo=tz)` | `DateTime::new(2026, 10, 24, &tz)?.with_time(8, 30, 15, 250)?` |
| `datetime.now(tz)` | `DateTime::now(&tz)` |
| `datetime.fromtimestamp(seconds, tz)` | `DateTime::from_timestamp_micros(micros, &tz)?` |
| `dt.replace(hour=9, minute=0, second=0, microsecond=0)` | `dt.with_time(9, 0, 0, 0)?` |
| `dt.replace(fold=1)` | `dt.with_fold(true)` |
| `dt.replace(tzinfo=tz)` | `dt.with_tz(&tz)` |

### Reading one

| Python | wallclock |
|---|---|
| `dt.year`, `dt.month`, `dt.day` | `dt.year()`, `dt.month()`, `dt.day()` |
| `dt.hour`, `dt.minute`, `dt.second`, `dt.microsecond` | `dt.hour()`, `dt.minute()`, `dt.second()`, `dt.microsecond()` |
| `dt.weekday()` | `dt.weekday()` |
| `dt.fold` | `dt.fold()` |
| `dt.tzinfo` | `dt.tz()` |
| `dt.utcoffset()` | `dt.utcoffset()` |
| `dt.tzname()` | `dt.tzname()` |
| `dt.timestamp()` | `dt.timestamp()`; exact: `dt.timestamp_micros()` |
| `dt.isoformat()` | `dt.isoformat()` |
| `str(dt)` | `dt.to_string()` |

### Arithmetic and comparison

| Python | wallclock |
|---|---|
| `dt + delta` | `dt.checked_add(delta)?` |
| `dt - delta` | `dt.checked_sub(delta)?` |
| `a - b` | `a.since(&b)` |
| `a < b`, `a <= b`, `a > b` | `a.py_cmp(&b)` (an `Ordering`) |
| `a == b` | `a.py_eq(&b)` |
| `dt.astimezone(tz)` | `dt.astimezone(&tz)?` |

### Timedeltas

| Python | wallclock |
|---|---|
| `timedelta(days=1)`, `timedelta(hours=1)` | `TimeDelta::DAY`, `TimeDelta::HOUR` |
| `timedelta(days=d, seconds=s, microseconds=us)` | `TimeDelta::new(d, s, us)?` |
| `timedelta(hours=n)` | `TimeDelta::from_hours(n)?` |
| `timedelta(minutes=n)` | `TimeDelta::from_minutes(n)?` |
| `timedelta(seconds=n)` | `TimeDelta::from_seconds(n)?` |
| `delta.days`, `delta.seconds`, `delta.microseconds` | `delta.days()`, `delta.seconds()`, `delta.microseconds()` |
| `delta.total_seconds()` | `delta.total_seconds()` |
| `a + b`, `a - b`, `a * n`, `-a` | `a.checked_add(b)?`, `a.checked_sub(b)?`, `a.checked_mul(n)?`, `a.checked_neg()?` |
| `timedelta.min`, `timedelta.max`, `timedelta.resolution` | `TimeDelta::MIN`, `TimeDelta::MAX`, `TimeDelta::MICROSECOND` |
| `str(delta)` | `delta.to_string()` |

## Where it looks different

**Operators are methods.** Python's `+`, `-`, `<` and `==` can raise or follow special rules, so here they are named methods. `checked_add` returns a `Result` because the sum can leave years 1 to 9999. `py_cmp` and `py_eq` carry Python's rule (wall clocks within a zone, instants across zones).

**`==` is not Python's `==`.** The `==` operator on a `DateTime` compares the wall-clock reading, the zone and `fold`, like comparing two structs. Python's `==` is `py_eq`. They differ for two datetimes that are the same moment in different zones (`py_eq` true, `==` false) and for the two readings of a repeated hour in one zone (`py_eq` true, `==` false).

**Exceptions are `Result`s.** Where Python raises, wallclock returns `Err`. `Error::python_exception()` names the class Python would have raised (`OverflowError`, `ValueError`, `ZoneInfoNotFoundError`). The message text is this crate's own.

**"The same tzinfo" means equal zones.** Python compares two datetimes by their wall clocks when they share one `tzinfo` object. Rust has no object identity, so wallclock uses equality: two zones are the same when they have the same key or the same fixed offset. The results agree with Python's, because two fixed offsets that are equal give the same answer either way.

**Zones are passed by reference.** `DateTime::new`, `from_timestamp_micros`, `now`, `astimezone` and `with_tz` take `&Tz` rather than moving the zone. A `Tz` is cheap to clone inside the library, so you can reuse one zone across many datetimes without cloning it yourself.

**Fields are set in two steps.** `DateTime::new` takes the date and gives midnight; `with_time` sets the time of day. Python does both in one call.

## Not available

| Python | Here |
|---|---|
| Naive datetimes (no `tzinfo`) | Every `DateTime` has a zone. Use `Tz::UTC` if you have nothing better. |
| `fromisoformat`, `strptime` | No parsing. Build from parts or from a timestamp. |
| `strftime` | Only `isoformat` and `Display`. |
| `dst()` | Not provided. `utcoffset()` and `tzname()` are. |
| `date`, `time` | Only the combined `DateTime`. |
| `datetime.fromtimestamp(float)` | Timestamps go in as whole microseconds, which is exact. |
