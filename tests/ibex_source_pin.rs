//! Source pin for the Ibex decoder fixtures.
//!
//! `ibex/rv32imcb.xif.yaml` and `ibex/rv32imcb_imm.xif.yaml` state that they
//! model the `OPCODE_OP` and `OPCODE_OP_IMM` decoding of
//! `ibex/rtl/ibex_decoder.sv`. Before issue #58 neither the file nor a
//! revision was pinned, so a decoder change between the transcription and
//! today could not be detected: the counts asserted in `run.sh` pin the
//! fixtures' own behaviour, not the decoder they claim to model.
//!
//! `tests/fixtures/ibex/decoder_pin.json` holds that pin: the source path,
//! the commit, the branch, the date, the file digest, the configuration the
//! fixtures assume, and the fixtures the pin covers.
//!
//! Two checks:
//!
//! - The pinned fixture files must still be the ones the pin was taken
//!   against. This runs everywhere, CI included, and it is a tripwire rather
//!   than a verification: editing a fixture means the transcription was
//!   touched, and the pin's claim only holds for the file that was read.
//! - When a checkout is at hand (`IBEX_DIR`, default `../ibex`), the decoder
//!   file is compared with the pin. The digest is the hard check. A checkout
//!   at another commit whose `ibex_decoder.sv` has the same digest describes
//!   the same decoding, so the revision is reported and not failed; a
//!   different digest fails naming both revisions.
//!
//! What this does not do: compare the fixtures' `(funct7, funct3)` mapping
//! with the decoder. The decoder states legality as behavioural `casez` code
//! with parameter-gated branches rather than as a table, so deriving the
//! accepted pairs from it is its own subject (issue #62). The fixture headers
//! say `transcribed from` for that reason.

use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Committed pin for the Ibex decoder fixtures.
const PIN_PATH: &str = "tests/fixtures/ibex/decoder_pin.json";
/// Default sibling Ibex checkout, relative to the crate root.
const DEFAULT_CHECKOUT: &str = "../ibex";
/// Environment variable overriding the checkout location.
const CHECKOUT_ENV: &str = "IBEX_DIR";

/// The decoder file and the fixtures that were transcribed from it.
#[derive(Debug, Deserialize)]
struct DecoderPin {
    /// Source file holding the decoder, relative to the checkout root.
    source_path: String,
    /// Revision the transcription was taken at.
    source_commit: String,
    /// Date of that revision.
    source_commit_date: String,
    /// Branch that revision sat on.
    source_branch: String,
    /// SHA-256 of the source file.
    source_sha256: String,
    /// Configuration the fixtures assume.
    config: String,
    /// Fixtures the pin covers.
    fixtures: Vec<PinnedFixture>,
}

/// One fixture the pin covers.
#[derive(Debug, Deserialize)]
struct PinnedFixture {
    /// Path of the fixture.
    path: String,
    /// SHA-256 of the fixture as the pin was taken.
    sha256: String,
}

fn load_pin() -> DecoderPin {
    let text = std::fs::read_to_string(PIN_PATH)
        .unwrap_or_else(|e| panic!("failed to read {PIN_PATH}: {e}"));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("failed to parse {PIN_PATH}: {e}"))
}

fn checkout_dir() -> PathBuf {
    match std::env::var(CHECKOUT_ENV) {
        Ok(dir) if !dir.trim().is_empty() => PathBuf::from(dir),
        _ => PathBuf::from(DEFAULT_CHECKOUT),
    }
}

fn git_head(dir: &Path) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let bytes =
        std::fs::read(path).map_err(|e| format!("failed to read {}: {e}", path.display()))?;
    Ok(hex::encode(Sha256::digest(&bytes)))
}

/// Channel 1: the pin still covers the fixtures it was taken against.
///
/// This runs with or without a checkout. It does not check the mapping, which
/// no gate does yet (issue #62); it makes a fixture edit visible, because the
/// pin's claim about the decoder only holds for the file that was read.
#[test]
fn pin_covers_the_fixture_files() {
    let pin = load_pin();
    let mut failures = Vec::new();
    for fixture in &pin.fixtures {
        let path = Path::new(&fixture.path);
        match sha256_file(path) {
            Err(message) => failures.push(format!("the pin names {}: {message}", fixture.path)),
            Ok(digest) if digest != fixture.sha256 => failures.push(format!(
                "{} has digest {digest}, but the pin records {}. The pin was taken against\n  \
                 {} at {} ({} {}, {}).\n  \
                 Read the decoder again and update the digest in {PIN_PATH}, so the pin\n  \
                 states what was checked.",
                fixture.path,
                fixture.sha256,
                pin.source_path,
                pin.source_commit,
                pin.source_branch,
                pin.source_commit_date,
                pin.config
            )),
            Ok(_) => eprintln!("{} matches the pin", fixture.path),
        }
    }
    assert!(
        failures.is_empty(),
        "the pin no longer covers the fixtures it was taken against:\n{}",
        failures.join("\n")
    );
}

/// Channel 2: when a checkout is at hand, the decoder file matches the pin.
#[test]
fn ibex_decoder_matches_the_pin() {
    let pin = load_pin();
    let dir = checkout_dir();
    let source = dir.join(&pin.source_path);
    if !source.is_file() {
        eprintln!(
            "no Ibex checkout at {} (set {CHECKOUT_ENV} to one with {}); the source channel did not run",
            dir.display(),
            pin.source_path
        );
        return;
    }

    let digest = sha256_file(&source).unwrap_or_else(|e| panic!("{e}"));
    let commit = git_head(&dir);
    let revision = commit.as_deref().unwrap_or("an unknown revision");
    assert_eq!(
        digest,
        pin.source_sha256,
        "the decoder at {revision} has digest {digest}, but the pin records {} for {} at {} ({} {}, {})",
        pin.source_sha256,
        source.display(),
        pin.source_commit,
        pin.source_branch,
        pin.source_commit_date,
        pin.config
    );

    match commit.as_deref() {
        Some(head) if head == pin.source_commit => {
            eprintln!("Ibex checkout {} at {head} matches the pin", dir.display());
        }
        Some(head) => eprintln!(
            "note: the checkout at {} is at {head}, not the pinned {}; {} is unchanged, so the decoding the fixtures describe is the same",
            dir.display(),
            pin.source_commit,
            pin.source_path
        ),
        None => eprintln!(
            "note: {} is not a git work tree; the revision was not compared with the pin",
            dir.display()
        ),
    }
}
