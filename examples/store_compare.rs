//! The same 828,000-point space held as a coordinate store and in SQLite.
//!
//! Both sides are Rust and run in one process, so neither pays another
//! language's overhead. The space is the one `examples/smr_cases/study.xif.yaml`
//! declares: 2,000 variants, 24 load cases, 5 regions, 3 materials, 2 methods,
//! with the two rules applied.
//!
//! The coordinate store is dense over the declared product: an address is
//! arithmetic on the axis values, so no key is stored and no index exists. It
//! holds one u16 payload a slot, which is what a lookup returns. SQLite stores
//! the key columns and the same payload, which is why the size row differs.
//!
//! Probe tuples are built before any timing, so neither side pays for
//! generating them inside the measured loop, and each SQLite measurement runs
//! one warm query first.
//!
//!     cargo run --release --example store_compare --features store-compare

use rusqlite::{params, Connection};
use std::hint::black_box;
use std::time::Instant;

const VARIANTS: usize = 2000;
const LOAD_CASES: usize = 24;
const REGIONS: usize = 5;
const MATERIALS: usize = 3;
const METHODS: usize = 2;
const DECLARED: usize = VARIANTS * LOAD_CASES * REGIONS * MATERIALS * METHODS;
/// A full scan is slow by definition, so a handful of probes is enough to
/// average it; ten thousand would spend minutes making the point.
const SCAN_PROBES: usize = 20;

/// The region to material rule from the spec.
fn material_allowed(region: usize, material: usize) -> bool {
    match region {
        0 | 4 => material != 1,
        1 | 2 => material != 2,
        3 => material == 2,
        _ => false,
    }
}

/// The load case to method rule from the spec.
fn method_allowed(load_case: usize, method: usize) -> bool {
    if load_case == 5 || load_case == 6 {
        method == 1
    } else {
        true
    }
}

/// The address of one combination in the declared product.
fn address(probe: [usize; 5]) -> usize {
    let [variant, load_case, region, material, method] = probe;
    (((variant * LOAD_CASES + load_case) * REGIONS + region) * MATERIALS + material) * METHODS
        + method
}

struct Row {
    variant: usize,
    load_case: usize,
    region: usize,
    material: usize,
    method: usize,
}

fn admitted() -> Vec<Row> {
    let mut rows = Vec::with_capacity(828_000);
    for variant in 0..VARIANTS {
        for load_case in 0..LOAD_CASES {
            for region in 0..REGIONS {
                for material in 0..MATERIALS {
                    if !material_allowed(region, material) {
                        continue;
                    }
                    for method in 0..METHODS {
                        if !method_allowed(load_case, method) {
                            continue;
                        }
                        rows.push(Row {
                            variant,
                            load_case,
                            region,
                            material,
                            method,
                        });
                    }
                }
            }
        }
    }
    rows
}

fn mib(bytes: u64) -> f64 {
    bytes as f64 / (1024.0 * 1024.0)
}

fn main() -> rusqlite::Result<()> {
    let rows = admitted();
    assert_eq!(rows.len(), 828_000, "the spec admits 828,000 combinations");
    println!("points                     {}\n", rows.len());

    // Probes are built before anything is timed.
    let five_probes: Vec<[usize; 5]> = (0..10_000)
        .map(|probe| {
            [
                probe % VARIANTS,
                probe % LOAD_CASES,
                probe % REGIONS,
                probe % MATERIALS,
                probe % METHODS,
            ]
        })
        .collect();
    let subset_probes: Vec<[usize; 2]> = (0..SCAN_PROBES)
        .map(|probe| [probe % REGIONS, probe % MATERIALS])
        .collect();
    black_box(&five_probes);
    black_box(&subset_probes);

    // ── coordinate store ───────────────────────────────────────────────
    let started = Instant::now();
    let mut store = vec![0u16; DECLARED];
    for row in &rows {
        let at = address([
            row.variant,
            row.load_case,
            row.region,
            row.material,
            row.method,
        ]);
        store[at] = at as u16;
    }
    let coordinate_fill = started.elapsed();
    let coordinate_size = (DECLARED * std::mem::size_of::<u16>()) as u64;

    // One address computation and one read a probe, repeated for a stable mean.
    let passes = 100;
    let started = Instant::now();
    let mut hits = 0usize;
    for _ in 0..passes {
        for probe in &five_probes {
            let at = address(*probe);
            hits += usize::from(black_box(store[at]) != 0);
        }
    }
    let coordinate_lookup =
        started.elapsed().as_nanos() as f64 / (passes * five_probes.len()) as f64;

    // A subset has no address to compute, so the coordinate side scans the
    // declared space and reads the axes out of each address.
    let started = Instant::now();
    let mut found = 0usize;
    for probe in &subset_probes {
        let (region, material) = (probe[0], probe[1]);
        for at in 0..DECLARED {
            if black_box(store[at]) == 0 {
                continue;
            }
            let at_region = (at / METHODS / MATERIALS) % REGIONS;
            let at_material = (at / METHODS) % MATERIALS;
            found += usize::from(at_region == region && at_material == material);
        }
    }
    let coordinate_subset = started.elapsed().as_secs_f64() * 1e3 / subset_probes.len() as f64;

    // ── sqlite ─────────────────────────────────────────────────────────
    let plain = sqlite("plain", &rows, &[])?;
    let two = sqlite(
        "two-indexes",
        &rows,
        &[
            "CREATE INDEX all_five ON t (variant, load_case, region, material, method)",
            "CREATE INDEX variant_load ON t (variant, load_case)",
        ],
    )?;
    let three = sqlite(
        "three-indexes",
        &rows,
        &[
            "CREATE INDEX all_five ON t (variant, load_case, region, material, method)",
            "CREATE INDEX variant_load ON t (variant, load_case)",
            "CREATE INDEX region_material ON t (region, material)",
        ],
    )?;

    let five = "SELECT payload FROM t WHERE variant=?1 AND load_case=?2 AND region=?3 \
                AND material=?4 AND method=?5";
    let subset = "SELECT payload FROM t WHERE region=?1 AND material=?2";

    let sqlite_five_plain = timed_five(&plain.0, five, &five_probes, SCAN_PROBES)?;
    let sqlite_five_two = timed_five(&two.0, five, &five_probes, five_probes.len())?;
    let sqlite_subset_two = timed_subset(&two.0, subset, &subset_probes)?;
    let sqlite_subset_three = timed_subset(&three.0, subset, &subset_probes)?;

    black_box((hits, found, sqlite_five_plain, sqlite_five_two));

    println!(
        "{:<26} {:>14} {:>12} {:>14} {:>15}",
        "", "coordinate", "sqlite", "sqlite + 2 idx", "sqlite + 3 idx"
    );
    println!(
        "{:<26} {:>11.1} MiB {:>8.1} MiB {:>11.1} MiB {:>12.1} MiB",
        "size",
        mib(coordinate_size),
        mib(plain.1),
        mib(two.1),
        mib(three.1)
    );
    println!(
        "{:<26} {:>14} {:>12} {:>14} {:>15}",
        "load, insert",
        format!("{:.0} ms", coordinate_fill.as_secs_f64() * 1e3),
        format!("{:.1} s", plain.2),
        format!("{:.1} s", two.2),
        format!("{:.1} s", three.2)
    );
    println!(
        "{:<26} {:>14} {:>12} {:>14} {:>15}",
        "load, build indexes",
        "none",
        "none",
        format!("{:.1} s", two.3),
        format!("{:.1} s", three.3)
    );
    println!(
        "{:<26} {:>14} {:>12} {:>14} {:>15}",
        "query, five axes",
        format!("{:.1} ns", coordinate_lookup),
        format!("{:.1} ms", sqlite_five_plain / 1e3),
        format!("{:.1} us", sqlite_five_two),
        format!("{:.1} us", sqlite_five_two)
    );
    println!(
        "{:<26} {:>14} {:>12} {:>14} {:>15}",
        "query, region+material",
        format!("{:.1} ms", coordinate_subset),
        format!("{:.1} ms", sqlite_subset_two),
        format!("{:.1} ms", sqlite_subset_two),
        format!("{:.1} us", sqlite_subset_three)
    );
    println!();
    println!(
        "indexes needed for every subset of 5 axes: {}",
        (1 << 5) - 1
    );
    println!("the two-index database covers 3 of those subsets; the three-index one covers 4");
    println!("the coordinate array holds {DECLARED} u16, payload only; the axes are the address");
    println!("the SQLite files hold the key columns and the payload");
    Ok(())
}

/// One query a probe, averaged; the first query runs before the timer so the
/// page cache is warm on both sides.
fn timed_five(
    connection: &Connection,
    sql: &str,
    probes: &[[usize; 5]],
    iterations: usize,
) -> rusqlite::Result<f64> {
    let mut statement = connection.prepare_cached(sql)?;
    let mut rows = statement.query(params![
        probes[0][0],
        probes[0][1],
        probes[0][2],
        probes[0][3],
        probes[0][4]
    ])?;
    while rows.next()?.is_some() {}
    drop(rows);
    drop(statement);

    let mut statement = connection.prepare_cached(sql)?;
    let started = Instant::now();
    for probe in probes.iter().take(iterations) {
        let mut found =
            statement.query(params![probe[0], probe[1], probe[2], probe[3], probe[4]])?;
        while found.next()?.is_some() {}
    }
    let elapsed = started.elapsed().as_secs_f64() * 1e6;
    Ok(elapsed / probes.iter().take(iterations).count() as f64)
}

/// The subset query, averaged over its probes, with one warm query first.
fn timed_subset(
    connection: &Connection,
    sql: &str,
    probes: &[[usize; 2]],
) -> rusqlite::Result<f64> {
    let mut statement = connection.prepare_cached(sql)?;
    let mut rows = statement.query(params![probes[0][0], probes[0][1]])?;
    while rows.next()?.is_some() {}
    drop(rows);
    drop(statement);

    let mut statement = connection.prepare_cached(sql)?;
    let started = Instant::now();
    for probe in probes {
        let mut found = statement.query(params![probe[0], probe[1]])?;
        while found.next()?.is_some() {}
    }
    let elapsed = started.elapsed().as_secs_f64() * 1e3;
    Ok(elapsed / probes.len() as f64)
}

/// One SQLite database: insert the admitted rows, create the indexes, and
/// report the file size with the insert and index times kept apart.
fn sqlite(
    name: &str,
    rows: &[Row],
    indexes: &[&str],
) -> rusqlite::Result<(Connection, u64, f64, f64)> {
    let path = format!("/tmp/ev-store-compare-{name}.db");
    let _ = std::fs::remove_file(&path);
    let connection = Connection::open(&path)?;
    connection.execute_batch(
        "PRAGMA journal_mode=OFF; CREATE TABLE t \
         (variant, load_case, region, material, method, payload)",
    )?;

    let started = Instant::now();
    {
        let transaction = connection.unchecked_transaction()?;
        let mut statement = transaction.prepare("INSERT INTO t VALUES (?1, ?2, ?3, ?4, ?5, ?6)")?;
        for row in rows {
            statement.execute(params![
                row.variant as u16,
                row.load_case as u16,
                row.region as u16,
                row.material as u16,
                row.method as u16,
                address([
                    row.variant,
                    row.load_case,
                    row.region,
                    row.material,
                    row.method
                ]) as u16
            ])?;
        }
        drop(statement);
        transaction.commit()?;
    }
    let insert = started.elapsed().as_secs_f64();

    let started = Instant::now();
    for index in indexes {
        connection.execute_batch(index)?;
    }
    let build = started.elapsed().as_secs_f64();

    let size = std::fs::metadata(&path).map(|meta| meta.len()).unwrap_or(0);
    Ok((connection, size, insert, build))
}
