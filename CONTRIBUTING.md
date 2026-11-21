# Contributing

wallclock has one rule: **it does what CPython does.** A change is right when CPython gives the same answer, and it is tested by asking CPython.

## Setting up

You need a Rust toolchain (1.85 or newer) and Python 3.11 or newer. On Windows, also `pip install tzdata`, since Windows has no time zone database of its own.

```sh
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

## How the tests work

`tests/record_vectors.py` asks CPython questions and writes its answers to `tests/vectors/cpython.json`. `tests/cpython.rs` asks this crate the same questions and expects the same answers. Each line of the file is one question:

| Kind | Asks |
|---|---|
| `describe` | the offset, zone name, timestamp and text of one reading |
| `add` | a reading plus a timedelta |
| `astimezone` | a reading moved to another zone |
| `compare` | `<`, `==` and `-` between two readings |
| `fromutc` | the reading of an instant in a zone |
| `timedelta` | the parts, total seconds and text of a timedelta |
| `zone` | whether a zone key is accepted, and the error if not |

The examples in the README, the guide and the API documentation are tests too: `cargo test` compiles and runs them.

## Changing behaviour

1. Find what CPython does. The readable reference is `Lib/_pydatetime.py` and `Lib/zoneinfo/_zoneinfo.py` in the CPython source.
2. Add cases to `tests/record_vectors.py` that show it. Aim at the awkward inputs: a repeated hour, a skipped hour, both ends of the calendar, an offset with seconds in it.
3. Record and run:

   ```sh
   python tests/record_vectors.py
   cargo test
   ```

4. Change the code until the test passes. Do not edit `cpython.json` by hand, and do not drop a case because it fails.

Zone cases in the recorded set keep to zones and years whose rules are long settled, so the file gives the same answers against any recent copy of the time zone database. Check this before sending a change: record on one system and run `cargo test` on another, or let CI do it (it runs Linux, macOS and Windows).

## The sweep

The recorded set covers 13 zones. The sweep covers every zone in the database over its whole history, about 750,000 questions. It is too big to keep in the repository and is only meaningful where Python and this crate read the same copy of the database, so it is recorded on the spot:

```sh
python tests/record_vectors.py --sweep /tmp/sweep.jsonl
SWEEP=/tmp/sweep.jsonl cargo test --test cpython -- --ignored
```

Run it on Linux or macOS after any change to `src/tz.rs` or `src/datetime.rs`. CI runs it on every push.

## Does a test catch it?

Before trusting a new rule, break it on purpose and check that `cargo test` fails. The recorded set has caught every deliberate break tried so far, among them: reading a skipped hour by the wrong side's rules, never setting `fold` on conversion, adding a day to the instant instead of the wall clock, and comparing two datetimes of one zone by instant.

## Style

- Public items have a doc comment that says what the thing is for and names its Python counterpart. New public functions get a short example.
- No `unsafe`.
- Errors are `Error` values named after Python's exception. No panics on any input a caller can build.
- Plain words in comments and docs. Say why, not what.

## Releasing

1. Update `CHANGELOG.md` and the version in `Cargo.toml`.
2. `cargo publish --dry-run`.
3. Tag `vX.Y.Z` and push the tag.
4. `cargo publish`.
