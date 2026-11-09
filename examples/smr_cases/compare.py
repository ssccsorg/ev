#!/usr/bin/env python3
"""Compare the case space held as a coordinate array with the same set in SQLite.

The point set is the one `study.xif.yaml` declares: 2,000 variants, 24 load
cases, 5 regions, 3 materials, 2 methods, with the two rules applied. The count
is checked against the 828,000 the engine reports.

The coordinate side is a row width, not a store: five u16 axis values and one
flag byte, which is what a CoordSpace holds. Its query cost is the arithmetic
in the key and is not timed here; the Rust bench in the primitive paper reports
0.38 ns for a dense lookup. SQLite is timed, with Python's call overhead
included, so its figures are upper bounds.

    python3 compare.py
"""

import os
import sqlite3
import time

REGION_MATERIAL = {0: [0, 2], 1: [0, 1], 2: [0, 1], 3: [2], 4: [0, 2]}
TEST_CASES = {5, 6}
AXES = 5
QUERIES = 5000


def points():
    rows = []
    for variant in range(2000):
        for load_case in range(24):
            for region in range(5):
                for material in REGION_MATERIAL[region]:
                    for method in ([1] if load_case in TEST_CASES else [0, 1]):
                        rows.append((variant, load_case, region, material, method))
    return rows


def timed_insert(path, indexes):
    if os.path.exists(path):
        os.remove(path)
    started = time.perf_counter()
    db = sqlite3.connect(path)
    db.execute("CREATE TABLE t (variant, load_case, region, material, method)")
    db.executemany("INSERT INTO t VALUES (?,?,?,?,?)", ROWS)
    for statement in indexes:
        db.execute(statement)
    db.commit()
    elapsed = time.perf_counter() - started
    return db, elapsed, os.path.getsize(path)


def timed_query(db, sql, args, count):
    cursor = db.cursor()
    started = time.perf_counter()
    for index in range(count):
        cursor.execute(sql, args(index))
        cursor.fetchall()
    return (time.perf_counter() - started) / count * 1e6


ROWS = points()
assert len(ROWS) == 828_000, f"the engine reports 828,000 points, this made {len(ROWS)}"

coordinate = len(ROWS) * (AXES * 2 + 1) / 1024 / 1024

plain, plain_insert, plain_size = timed_insert("/tmp/ev-compare-plain.db", [])
indexed, indexed_insert, indexed_size = timed_insert(
    "/tmp/ev-compare-indexed.db",
    [
        "CREATE INDEX all_five ON t (variant, load_case, region, material, method)",
        "CREATE INDEX variant_load ON t (variant, load_case)",
    ],
)

five = "SELECT method FROM t WHERE variant=? AND load_case=? AND region=? AND material=?"
two = "SELECT method FROM t WHERE region=? AND material=?"

plain_five = timed_query(plain, five, lambda i: (i % 2000, i % 24, i % 5, i % 3), 20)
plain_two = timed_query(plain, two, lambda i: (i % 5, i % 3), 20)
indexed_five = timed_query(indexed, five, lambda i: (i % 2000, i % 24, i % 5, i % 3), QUERIES)
indexed_two = timed_query(indexed, two, lambda i: (i % 5, i % 3), 20)

print(f"points                     {len(ROWS):,}")
print()
print(f"{'':26} {'coordinate':>16} {'sqlite':>10} {'sqlite + 2 idx':>16}")
print(
    f"{'size':26} {coordinate:12.1f} MiB {plain_size / 1024 / 1024:7.1f} MiB {indexed_size / 1024 / 1024:13.1f} MiB"
)
print(
    f"{'load':26} {'sequential':>16} {plain_insert:7.1f} s {indexed_insert:13.1f} s"
)
print(
    f"{'query, five axes':26} {'key (~0.4 ns)':>16} {plain_five / 1000:7.1f} ms {indexed_five:13.1f} us"
)
print(
    f"{'query, region+material':26} {'key (~1.4 ns)':>16} {plain_two / 1000:7.1f} ms {indexed_two / 1000:13.1f} ms"
)
print()
print(f"indexes needed for every subset of {AXES} axes: {2 ** AXES - 1}")
print("indexes above: 2, covering 3 of those subsets")
print("sqlite figures include Python's call overhead, so they are upper bounds")
print("coordinate figures are the Rust benches: dense lookup 0.38-0.40 ns, axis decode 1.44 ns")
