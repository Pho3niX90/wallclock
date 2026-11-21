//! Every answer CPython gave, asked of this crate.
//!
//! `tests/vectors/cpython.json` is written by `tests/record_vectors.py`.

use std::cmp::Ordering;

use serde_json::Value;
use wallclock::{DateTime, Error, TimeDelta, Tz};

fn tz(spec: &str) -> Tz {
    match spec.strip_prefix("fixed:") {
        Some(micros) => {
            let micros: i128 = micros.parse().unwrap();
            Tz::fixed_delta(TimeDelta::from_micros(micros).unwrap()).unwrap()
        }
        None => Tz::zone(spec).unwrap(),
    }
}

fn int(value: &Value) -> i64 {
    value
        .as_i64()
        .unwrap_or_else(|| panic!("not an integer: {value}"))
}

/// `[year, month, day, hour, minute, second, microsecond, fold, zone]`.
fn datetime(fields: &Value) -> DateTime {
    let field = |index: usize| int(&fields[index]) as u32;
    DateTime::new(
        int(&fields[0]) as i32,
        field(1),
        field(2),
        &tz(fields[8].as_str().unwrap()),
    )
    .unwrap()
    .with_time(field(3), field(4), field(5), field(6))
    .unwrap()
    .with_fold(field(7) == 1)
}

fn delta(triple: &Value) -> Result<TimeDelta, Error> {
    TimeDelta::new(int(&triple[0]), int(&triple[1]), int(&triple[2]))
}

/// A result as the vectors hold it: the fields, or the exception's name.
fn shown(result: Result<DateTime, Error>) -> Value {
    match result {
        Ok(value) => serde_json::json!([
            value.year(),
            value.month(),
            value.day(),
            value.hour(),
            value.minute(),
            value.second(),
            value.microsecond(),
            u8::from(value.fold()),
            value.tz().key().map_or_else(
                || format!(
                    "fixed:{}",
                    value.tz().fixed_offset().unwrap().total_microseconds()
                ),
                str::to_owned
            ),
        ]),
        Err(error) => Value::String(error.python_exception().to_owned()),
    }
}

fn check(case: &Value) -> Result<(), String> {
    let differs = |what: &str, got: &dyn std::fmt::Debug, expected: &dyn std::fmt::Debug| {
        Err(format!("{what}: got {got:?}, CPython gave {expected:?}"))
    };
    match case[0].as_str().unwrap() {
        "describe" => {
            let value = datetime(&case[1]);
            let got = (
                value.utcoffset().total_microseconds() as i64,
                value.tzname(),
                value.timestamp_micros(),
                python_float(value.timestamp()),
                value.isoformat(),
                value.to_string(),
                i64::from(value.weekday()),
            );
            let expected = (
                int(&case[2]),
                case[3].as_str().unwrap().to_owned(),
                int(&case[4]),
                python_float(case[5].as_str().unwrap().parse::<f64>().unwrap()),
                case[6].as_str().unwrap().to_owned(),
                case[7].as_str().unwrap().to_owned(),
                int(&case[8]),
            );
            if got != expected {
                return differs("describe", &got, &expected);
            }
        }
        "zone" => {
            let key = case[1].as_str().unwrap();
            let got = match Tz::zone(key) {
                Ok(_) => "ok",
                Err(error) => error.python_exception(),
            };
            let expected = case[2].as_str().unwrap();
            if got != expected {
                return differs("zone", &got, &expected);
            }
        }
        "add" => {
            let value = datetime(&case[1]);
            let got = shown(delta(&case[2]).and_then(|delta| value.checked_add(delta)));
            if got != case[3] {
                return differs("add", &got, &case[3]);
            }
            // Subtracting the opposite is the same sum.
            if let Ok(opposite) = delta(&case[2]).and_then(TimeDelta::checked_neg) {
                let got = shown(value.checked_sub(opposite));
                if got != case[3] {
                    return differs("sub", &got, &case[3]);
                }
            }
        }
        "astimezone" => {
            let got = shown(datetime(&case[1]).astimezone(&tz(case[2].as_str().unwrap())));
            if got != case[3] {
                return differs("astimezone", &got, &case[3]);
            }
        }
        "compare" => {
            let (one, two) = (datetime(&case[1]), datetime(&case[2]));
            let order = match one.py_cmp(&two) {
                Ordering::Less => -1,
                Ordering::Equal => 0,
                Ordering::Greater => 1,
            };
            let got = (
                order,
                one.py_eq(&two),
                one.since(&two).total_microseconds() as i64,
            );
            let expected = (int(&case[3]), case[4].as_bool().unwrap(), int(&case[5]));
            if got != expected {
                return differs("compare", &got, &expected);
            }
            if two.py_eq(&one) != got.1 || two.py_cmp(&one) != one.py_cmp(&two).reverse() {
                return Err("the comparison is not symmetric".to_owned());
            }
        }
        "fromutc" => {
            let zone = tz(case[2].as_str().unwrap());
            let got = shown(DateTime::from_timestamp_micros(int(&case[1]), &zone));
            if got != case[3] {
                return differs("fromutc", &got, &case[3]);
            }
            // And back: the same instant.
            if let Ok(value) = DateTime::from_timestamp_micros(int(&case[1]), &zone) {
                if value.timestamp_micros() != int(&case[1]) {
                    return differs("round trip", &value.timestamp_micros(), &case[1]);
                }
            }
        }
        "timedelta" => {
            let got = match delta(&case[1]) {
                Ok(delta) => serde_json::json!([
                    delta.days(),
                    delta.seconds(),
                    delta.microseconds(),
                    python_float(delta.total_seconds()),
                    delta.to_string(),
                ]),
                Err(error) => Value::String(error.python_exception().to_owned()),
            };
            let mut expected = case[2].clone();
            // Compare the float as a float: Python's text for it is not Rust's.
            if let Some(text) = expected.get(3).and_then(Value::as_str) {
                expected[3] = python_float(text.parse().unwrap());
            }
            if got != expected {
                return differs("timedelta", &got, &expected);
            }
        }
        other => return Err(format!("unknown case kind {other}")),
    }
    Ok(())
}

fn python_float(value: f64) -> Value {
    Value::String(format!("{:016x}", value.to_bits()))
}

#[test]
fn every_recorded_answer_matches() {
    let text = include_str!("vectors/cpython.json");
    let document: Value = serde_json::from_str(text).unwrap();
    let cases = document["cases"].as_array().unwrap();
    assert!(cases.len() > 1000, "the vectors are missing");
    let mut failures = Vec::new();
    for case in cases {
        if let Err(problem) = check(case) {
            failures.push(format!("{case}\n    {problem}"));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} cases differ from CPython {} (tzdata {}):\n{}",
        failures.len(),
        cases.len(),
        document["python"],
        document["tzdata"],
        failures[..failures.len().min(25)].join("\n")
    );
}

/// The sweep: every zone of the database over the whole of its history,
/// about a million questions.
///
/// It is only meaningful where Python and this crate read the same copy of
/// the database, so it runs on request, on a system that has one:
///
/// ```sh
/// python tests/record_vectors.py --sweep /tmp/sweep.jsonl
/// SWEEP=/tmp/sweep.jsonl cargo test --test cpython -- --ignored
/// ```
#[test]
#[ignore = "needs a sweep file recorded on this machine"]
fn every_zone_matches_cpython() {
    use std::collections::BTreeMap;
    use std::io::BufRead;

    let path =
        std::env::var("SWEEP").expect("set SWEEP to the file record_vectors.py --sweep wrote");
    let file = std::io::BufReader::new(std::fs::File::open(&path).unwrap());
    let (mut total, mut failures, mut examples) =
        (0u64, BTreeMap::<String, u64>::new(), Vec::new());
    for line in file.lines() {
        let case: Value = serde_json::from_str(&line.unwrap()).unwrap();
        total += 1;
        // A zone this crate cannot load panics in `tz`: count it as a failure too.
        let outcome = std::panic::catch_unwind(|| check(&case))
            .unwrap_or_else(|_| Err("panicked".to_owned()));
        if let Err(problem) = outcome {
            let zone = case[1]
                .get(8)
                .or(case.get(2))
                .and_then(Value::as_str)
                .unwrap_or("?");
            *failures
                .entry(format!("{} {zone}", case[0].as_str().unwrap()))
                .or_default() += 1;
            if examples.len() < 40 {
                examples.push(format!("{case}\n    {problem}"));
            }
        }
    }
    assert!(
        total > 100_000,
        "the sweep file is too small: {total} cases"
    );
    let failed: u64 = failures.values().sum();
    assert!(
        failed == 0,
        "{failed} of {total} cases differ from CPython.\nBy kind and zone: {failures:?}\n{}",
        examples.join("\n")
    );
    println!("{total} cases match CPython");
}
