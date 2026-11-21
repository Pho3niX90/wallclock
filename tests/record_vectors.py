"""Records CPython's answers for the cases in tests/cpython.rs.

    python tests/record_vectors.py

Needs Python 3.11 or newer and, where the system has no time zone database
(Windows), the ``tzdata`` package. The cases are the same on every run.

    python tests/record_vectors.py --sweep FILE

writes a far larger set instead, over every zone and all of its history, for
the sweep test (see ``sweep`` below).

To stay true whichever copy of the database a machine has, zone cases keep
to zones and years whose rules have long been settled: 1971 to 2037, the far
future (which follows the rule a zone ends with), and the years before
standard time, when a zone runs on local mean time.
"""

from __future__ import annotations

import datetime as dt
import json
import random
import sys
from pathlib import Path
from zoneinfo import ZoneInfo

OUT = Path(__file__).parent / "vectors" / "cpython.json"
UTC = dt.timezone.utc
EPOCH = dt.datetime(1970, 1, 1, tzinfo=UTC)
MICRO = dt.timedelta(microseconds=1)

#: Daylight saving north and south, half-hour and 45-minute offsets, a
#: half-hour daylight saving shift, zones without daylight saving, one whose
#: standard time is the summer one (Dublin), and a day skipped at the date line.
ZONES = [
    "Europe/Madrid", "America/New_York", "Australia/Sydney", "Australia/Lord_Howe", "America/St_Johns",
    "Asia/Kolkata", "Asia/Kathmandu", "Africa/Johannesburg", "Europe/Dublin", "Pacific/Apia", "Asia/Tokyo",
    "UTC", "Etc/GMT+5",
]
FIXED = [0, 3600 * 10**6, -5 * 3600 * 10**6, 19800 * 10**6, -(86400 * 10**6 - 1), 86400 * 10**6 - 1, 1, -3723000004]
#: Zones on local mean time long ago, with seconds in the offset.
OLD = ["Europe/Madrid", "Asia/Tokyo", "America/New_York"]


def tz_of(spec: str) -> dt.tzinfo:
    if spec.startswith("fixed:"):
        return dt.timezone(dt.timedelta(microseconds=int(spec[6:])))
    return ZoneInfo(spec)


def row(value: dt.datetime, spec: str) -> list:
    return [value.year, value.month, value.day, value.hour, value.minute, value.second, value.microsecond, value.fold, spec]


def build(fields: list) -> dt.datetime:
    year, month, day, hour, minute, second, micro, fold, spec = fields
    return dt.datetime(year, month, day, hour, minute, second, micro, tzinfo=tz_of(spec), fold=fold)


def transitions(zone: str, first: int, last: int) -> list[dt.datetime]:
    """The instants (UTC) at which the zone's offset changes, to the second."""
    tz = ZoneInfo(zone)
    found = []
    at = dt.datetime(first, 1, 1, tzinfo=UTC)
    end = dt.datetime(last, 12, 31, tzinfo=UTC)
    step = dt.timedelta(days=1)
    while at < end:
        after = at + step
        if at.astimezone(tz).utcoffset() != after.astimezone(tz).utcoffset():
            low, high = at, after
            while high - low > dt.timedelta(seconds=1):
                middle = low + (high - low) // 2
                middle -= dt.timedelta(microseconds=middle.microsecond)
                if middle.astimezone(tz).utcoffset() == low.astimezone(tz).utcoffset():
                    low = middle
                else:
                    high = middle
            found.append(high)
        at = after
    return found


def wall_cases(rng: random.Random) -> list[list]:
    """Wall-clock readings worth asking about, as rows."""
    rows: list[list] = []
    around = [0, -1, 1800, -1800, 3599, 3600, -3600, 5400, 86400, -86400]
    for zone in ZONES:
        tz = ZoneInfo(zone)
        changes = transitions(zone, 1971, 2037) + transitions(zone, 2400, 2401) + transitions(zone, 9990, 9991)
        picked = changes if len(changes) <= 5 else rng.sample(changes, 5)
        for change in picked:
            # The wall clock just before the change, and readings either side of it.
            before = (change - dt.timedelta(seconds=1)).astimezone(tz).replace(tzinfo=None) + dt.timedelta(seconds=1)
            for seconds in around:
                wall = before + dt.timedelta(seconds=seconds, microseconds=rng.choice([0, 0, 1, 999999]))
                for fold in (0, 1):
                    rows.append(row(wall.replace(fold=fold), zone))
        for _ in range(12):
            year = rng.choice([rng.randint(1971, 2037), rng.randint(1971, 2037), rng.randint(2038, 9998)])
            wall = dt.datetime(year, rng.randint(1, 12), rng.randint(1, 28), rng.randint(0, 23), rng.randint(0, 59),
                               rng.randint(0, 59), rng.choice([0, 0, rng.randint(0, 999999)]))
            rows.append(row(wall.replace(fold=rng.choice([0, 0, 1])), zone))
    for zone in OLD:
        for year in (1, 2, 800, 1500, 1850):
            rows.append(row(dt.datetime(year, rng.randint(1, 12), rng.randint(1, 28), rng.randint(0, 23), 30, 15), zone))
    for offset in FIXED:
        spec = f"fixed:{offset}"
        for wall in [dt.datetime(1, 1, 1), dt.datetime(9999, 12, 31, 23, 59, 59, 999999), dt.datetime(1970, 1, 1),
                     dt.datetime(2026, 10, 25, 2, 30, fold=1), dt.datetime(2024, 2, 29, 12, 0, 0, 5)]:
            rows.append(row(wall, spec))
    for spec in ["UTC", "Asia/Tokyo", "Etc/GMT+5", "Europe/Madrid"]:
        rows.append(row(dt.datetime(1, 1, 1), spec))
        rows.append(row(dt.datetime(1, 1, 1, 12), spec))
        rows.append(row(dt.datetime(9999, 12, 31, 23, 59, 59, 999999), spec))
        rows.append(row(dt.datetime(9999, 12, 31, 6), spec))
    return rows


def outcome(call) -> object:
    try:
        return call()
    except OverflowError:
        return "OverflowError"


def main() -> None:
    rng = random.Random("wallclock")
    walls = wall_cases(rng)
    specs = ZONES + [f"fixed:{offset}" for offset in FIXED]
    cases: list[list] = []

    # Instants across the whole calendar, to the microsecond: timestamp() has
    # to divide exactly, where converting to a float first rounds twice.
    for _ in range(150):
        micros = rng.randint(-62_135_596_800 * 10**6, 253_402_300_799 * 10**6)
        walls.append(row(EPOCH + dt.timedelta(microseconds=micros), "fixed:0"))

    for fields in walls:
        value = build(fields)
        cases.append([
            "describe", fields, value.utcoffset() // MICRO, value.tzname(), (value - EPOCH) // MICRO,
            repr(value.timestamp()), value.isoformat(), str(value), value.weekday(),
        ])

    deltas = [(1, 0, 0), (-1, 0, 0), (0, 3600, 0), (0, -3600, 0), (0, 0, 1), (0, 0, -1), (7, 0, 0), (0, 1800, 500000),
              (365, 0, 0), (-366, 5, 5), (0, 0, 0), (3652059, 0, 0), (-3652059, 0, 0), (999999999, 0, 0),
              (-999999999, 0, 0)]
    for fields in walls:
        value = build(fields)
        for delta in rng.sample(deltas, 2) + [(1, 0, 0)]:
            added = outcome(lambda: value + dt.timedelta(*delta))
            cases.append(["add", fields, list(delta), added if isinstance(added, str) else row(added, fields[8])])

    for fields in walls:
        value = build(fields)
        for spec in rng.sample(specs, 2) + [fields[8]]:
            moved = outcome(lambda: value.astimezone(tz_of(spec)))
            cases.append(["astimezone", fields, spec, moved if isinstance(moved, str) else row(moved, spec)])

    for _ in range(len(walls)):
        first = rng.choice(walls)
        roll = rng.random()
        if roll < 0.35:
            second = rng.choice(walls)
        elif roll < 0.6:  # the same instant somewhere else, where it exists
            moved = outcome(lambda: build(first).astimezone(tz_of(rng.choice(specs))))
            second = first if isinstance(moved, str) else row(moved, moved.tzinfo.key if isinstance(moved.tzinfo, ZoneInfo)
                                                              else f"fixed:{moved.utcoffset() // MICRO}")
        elif roll < 0.8:  # the same zone, another reading
            second = [*rng.choice(walls)[:8], first[8]]
        else:  # the other fold of the same reading
            second = [*first[:7], 1 - first[7], first[8]]
        one, two = build(first), build(second)
        cases.append(["compare", first, second, (one > two) - (one < two), one == two, (one - two) // MICRO])

    for _ in range(250):
        spec = rng.choice(specs)
        year = rng.choice([rng.randint(1971, 2037), rng.randint(2038, 9998)])
        micros = (dt.datetime(year, rng.randint(1, 12), rng.randint(1, 28), rng.randint(0, 23), rng.randint(0, 59),
                              rng.randint(0, 59), rng.choice([0, 1, 999999]), tzinfo=UTC) - EPOCH) // MICRO
        moved = outcome(lambda: (EPOCH + dt.timedelta(microseconds=micros)).astimezone(tz_of(spec)))
        cases.append(["fromutc", micros, spec, moved if isinstance(moved, str) else row(moved, spec)])
    # Either side of a clock change: which of two readings an instant is.
    for zone in ZONES:
        changes = transitions(zone, 1971, 2037)
        for change in changes if len(changes) <= 6 else rng.sample(changes, 6):
            for seconds in (-1, 0, 1, 1800, 3599, 3600, 7200):
                micros = (change - EPOCH) // MICRO + seconds * 10**6
                moved = (EPOCH + dt.timedelta(microseconds=micros)).astimezone(ZoneInfo(zone))
                cases.append(["fromutc", micros, zone, row(moved, zone)])
    for spec in specs:
        for wall in [dt.datetime(1, 1, 1), dt.datetime(1, 1, 1, 23), dt.datetime(9999, 12, 31), dt.datetime(9999, 12, 31, 23, 59, 59, 999999)]:
            micros = (wall.replace(tzinfo=UTC) - EPOCH) // MICRO
            moved = outcome(lambda: (EPOCH + dt.timedelta(microseconds=micros)).astimezone(tz_of(spec)))
            cases.append(["fromutc", micros, spec, moved if isinstance(moved, str) else row(moved, spec)])

    triples = [(0, 0, 0), (1, 0, 0), (-1, 0, 0), (0, -1, 0), (0, 0, -1), (1, 7384, 5), (400, 86399, 0), (0, 86400, 0),
               (999999999, 86399, 999999), (-999999999, 0, 0), (1000000000, 0, 0), (-999999999, 0, -1), (0, 1, 500000),
               (0, 253402300799, 999999), (0, 0, 253402300799999999), (2, -90000, 3000000)]
    triples += [(rng.randint(-10**6, 10**6), rng.randint(-10**6, 10**6), rng.randint(-10**9, 10**9)) for _ in range(150)]
    triples += [(rng.randint(-10**9, 10**9), rng.randint(-10**11, 10**11), rng.randint(-10**17, 10**17)) for _ in range(150)]
    for triple in triples:
        delta = outcome(lambda: dt.timedelta(*triple))
        answer = delta if isinstance(delta, str) else [delta.days, delta.seconds, delta.microseconds,
                                                       repr(delta.total_seconds()), str(delta)]
        cases.append(["timedelta", list(triple), answer])

    # Zone keys: valid names, links, wrong case, malformed paths and unknown names.
    # Every key here has the same outcome on Windows and Linux.
    zone_keys = [
        "Europe/Madrid", "America/New_York", "UTC", "Etc/GMT+5",
        "US/Eastern", "GB", "Etc/UTC",
        "europe/madrid", "EUROPE/MADRID", "us/eastern",
        "", "/", "/Europe/Madrid", "Europe/Madrid/", "Europe//Madrid",
        ".", "..", "./UTC", "UTC/.", "../UTC", "UTC/..", "Europe/../Europe/Madrid",
        "Europe/Madrid\x00",
        "Mars/Olympus_Mons", "NoSuchZone",
    ]
    for key in zone_keys:
        try:
            ZoneInfo(key)
        except Exception as error:
            answer = type(error).__name__
        else:
            answer = "ok"
        cases.append(["zone", key, answer])

    import importlib.metadata

    try:
        tzdata = importlib.metadata.version("tzdata")
    except importlib.metadata.PackageNotFoundError:
        tzdata = "system"
    document = {"python": sys.version.split()[0], "tzdata": tzdata, "cases": cases}
    lines = ["{", f' "python": {json.dumps(document["python"])},', f' "tzdata": {json.dumps(tzdata)},', ' "cases": [']
    lines += [f"  {json.dumps(case, ensure_ascii=True, separators=(',', ':'))}," for case in cases]
    lines[-1] = lines[-1][:-1]
    lines += [" ]", "}"]
    OUT.write_text("\n".join(lines) + "\n", encoding="utf-8", newline="\n")
    print(f"{len(cases)} cases -> {OUT}")


def sweep(out: Path) -> None:
    """Every zone of the database, over the whole of its history.

    Too big to keep (about a million cases), and only meaningful where
    Python and the crate read the same copy of the database, which they do
    on a system that has one. See ``every_zone_matches_cpython`` in
    ``tests/cpython.rs``.
    """
    from zoneinfo import available_timezones
    from zoneinfo._zoneinfo import ZoneInfo as PurePythonZoneInfo  # its transition list is readable

    rng = random.Random("sweep")
    zones = sorted(available_timezones() - {"localtime", "Factory", "posixrules"})
    count = 0
    with out.open("w", encoding="utf-8", newline="\n") as lines:

        def emit(case: list) -> None:
            nonlocal count
            lines.write(json.dumps(case, ensure_ascii=True, separators=(",", ":")) + "\n")
            count += 1

        for zone in zones:
            tz = ZoneInfo(zone)
            listed = [EPOCH + dt.timedelta(seconds=second) for second in PurePythonZoneInfo(zone)._trans_utc
                      if -62_000_000_000 < second < 253_000_000_000]
            # The first and the last few, and a sample of the rest; then the years the closing rule governs.
            picked = listed if len(listed) <= 24 else listed[:4] + listed[-4:] + rng.sample(listed[4:-4], 16)
            picked += transitions(zone, 2400, 2400) + transitions(zone, 9990, 9990)
            walls: list[list] = []
            for change in picked:
                before = (change - dt.timedelta(seconds=1)).astimezone(tz).replace(tzinfo=None) + dt.timedelta(seconds=1)
                for seconds in (0, -1, 1800, -1800, 3600, -3600, 7200, 86400):
                    try:
                        wall = before + dt.timedelta(seconds=seconds, microseconds=rng.choice([0, 0, 1, 999999]))
                    except OverflowError:
                        continue
                    walls += [row(wall.replace(fold=fold), zone) for fold in (0, 1)]
                for seconds in (-1, 0, 1, 1800, 3599, 3600, 7200):
                    micros = (change - EPOCH) // MICRO + seconds * 10**6
                    moved = outcome(lambda: (EPOCH + dt.timedelta(microseconds=micros)).astimezone(tz))
                    emit(["fromutc", micros, zone, moved if isinstance(moved, str) else row(moved, zone)])
            for _ in range(20):
                wall = dt.datetime(rng.randint(1800, 2100), rng.randint(1, 12), rng.randint(1, 28), rng.randint(0, 23),
                                   rng.randint(0, 59), rng.randint(0, 59), rng.choice([0, rng.randint(0, 999999)]))
                walls.append(row(wall.replace(fold=rng.choice([0, 1])), zone))
            for fields in walls:
                value = build(fields)
                emit(["describe", fields, value.utcoffset() // MICRO, value.tzname(), (value - EPOCH) // MICRO,
                      repr(value.timestamp()), value.isoformat(), str(value), value.weekday()])
                for delta in ((1, 0, 0), rng.choice([(-1, 0, 0), (0, 3600, 0), (0, -1800, 1), (7, 0, 0), (-365, 0, 0)])):
                    added = outcome(lambda: value + dt.timedelta(*delta))
                    emit(["add", fields, list(delta), added if isinstance(added, str) else row(added, zone)])
                for spec in ("fixed:0", rng.choice(zones)):
                    moved = outcome(lambda: value.astimezone(tz_of(spec)))
                    emit(["astimezone", fields, spec, moved if isinstance(moved, str) else row(moved, spec)])
            for _ in range(40):
                first = rng.choice(walls)
                second = rng.choice([rng.choice(walls), [*first[:7], 1 - first[7], zone]])
                if rng.random() < 0.4:
                    other = rng.choice(zones)
                    moved = outcome(lambda: build(first).astimezone(ZoneInfo(other)))
                    second = second if isinstance(moved, str) else row(moved, other)
                one, two = build(first), build(second)
                emit(["compare", first, second, (one > two) - (one < two), one == two, (one - two) // MICRO])
    print(f"{count} cases over {len(zones)} zones -> {out}")


if __name__ == "__main__":
    if len(sys.argv) == 3 and sys.argv[1] == "--sweep":
        sweep(Path(sys.argv[2]))
    else:
        main()
