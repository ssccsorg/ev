//! Fixture derivation gate: the CVA6 fixtures against the hardware decoder
//! mask table.
//!
//! `cva6/xif_ref.xif.yaml`, `cva6/xif_ref_r4.xif.yaml`, and
//! `cva6/xif_madd.xif.yaml` state in their headers that they are derived from
//! the coprocessor issue-response mask table in
//! `core/cvxif_example/include/cvxif_instr_pkg.sv`, evaluated by
//! `instr_decoder.sv` as
//! `sel[i] = ((CoproInstr[i].mask & issue_req_i.instr) == CoproInstr[i].instr)`.
//! Nothing re-derived them, so a wrong field mapping, a missed entry, or a
//! field the decoder pins while the fixture leaves it free would have been
//! invisible.
//!
//! The table itself is committed as an extraction
//! (`tests/fixtures/cva6/mask_table.json`) with its source path, the pinned
//! commit, and the source digests. Committing the extraction keeps the
//! third-party file out of the repository and lets the gate run everywhere,
//! CI included.
//!
//! Two channels:
//!
//! - Derivation: for each derived fixture, the table's accepted words and the
//!   fixture's accepted set must agree once both are restricted to the
//!   fixture's decision axes. The precondition is asserted first: a table
//!   entry whose mask reaches a bit no declared field covers fails with that
//!   entry named, so the projection cannot silently drop a bit.
//! - Source: when a checkout is at hand (`CVA6_DIR`, default `../cva6`), the
//!   table is re-extracted and the commit, the file digests, and the entries
//!   must match the committed copy. An unavailable checkout is reported.
//!
//! `EV_UPDATE_MASK_TABLE=1` rewrites the committed extraction from a checkout.

use ev::spec::{ConstraintSpec, EncodingLayout, FieldSpec, ProjectorSpec, VerificationSpec};
use ev::verify::compose::expand_all;
use ev::verify::evaluate::evaluate_all;
use ev::verify::registry::{ConstraintRegistry, ProjectorRegistry};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Committed extraction of the decoder mask table.
const MASK_TABLE_PATH: &str = "tests/fixtures/cva6/mask_table.json";
/// Default sibling CVA6 checkout, relative to the crate root.
const DEFAULT_CHECKOUT: &str = "../cva6";
/// Environment variable overriding the checkout location.
const CHECKOUT_ENV: &str = "CVA6_DIR";
/// Environment variable enabling the regeneration path.
const UPDATE_ENV: &str = "EV_UPDATE_MASK_TABLE";
/// Instruction word width the table and the fixtures model.
const INSN_WIDTH: u32 = 32;
/// Source file holding the table, relative to a checkout root.
const SOURCE_PATH: &str = "core/cvxif_example/include/cvxif_instr_pkg.sv";
/// Source file holding the decoder that evaluates the table.
const DECODER_PATH: &str = "core/cvxif_example/instr_decoder.sv";

/// Fixtures whose headers state that they are derived from the mask table.
const DERIVED_FIXTURES: &[&str] = &[
    "tests/fixtures/cva6/xif_ref.xif.yaml",
    "tests/fixtures/cva6/xif_ref_r4.xif.yaml",
    "tests/fixtures/cva6/xif_madd.xif.yaml",
];

// ── The committed extraction ──────────────────────────────────────────

/// Encode `instr` and `mask` as `0x` hex in the committed JSON. Instruction
/// encodings are read as hex, and this file is reviewed by hand when the pin
/// moves.
mod hex_u32 {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(value: &u32, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&format!("{value:#010x}"))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u32, D::Error> {
        let text = String::deserialize(deserializer)?;
        u32::from_str_radix(text.trim_start_matches("0x"), 16).map_err(serde::de::Error::custom)
    }
}

/// The decoder mask table as commit-ready data.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct MaskTable {
    /// Source file holding the table, relative to the checkout root.
    source_path: String,
    /// Revision the extraction was taken at.
    source_commit: String,
    /// SHA-256 of the source file.
    source_sha256: String,
    /// Source file holding the decoder that evaluates the table.
    decoder_path: String,
    /// SHA-256 of the decoder file.
    decoder_sha256: String,
    /// Entry count `NbInstr` declares.
    nb_instr: usize,
    /// One entry per decoded instruction.
    entries: Vec<MaskEntry>,
}

/// One `CoproInstr` entry.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct MaskEntry {
    /// Opcode name, such as `ADD_MULTI`.
    opcode: String,
    /// Pattern the masked word must equal, written as hex.
    #[serde(with = "hex_u32")]
    instr: u32,
    /// Bits the decoder compares, written as hex.
    #[serde(with = "hex_u32")]
    mask: u32,
    /// `resp.writeback`.
    writeback: u32,
    /// `resp.register_read`, most significant bit first as written.
    register_read: [u32; 3],
}

fn load_committed_table() -> MaskTable {
    let text = std::fs::read_to_string(MASK_TABLE_PATH)
        .unwrap_or_else(|e| panic!("failed to read {MASK_TABLE_PATH}: {e}"));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("failed to parse {MASK_TABLE_PATH}: {e}"))
}

/// The revision the committed extraction was taken at, when a readable copy
/// exists. Best effort: the regeneration path only reports it, and it must
/// still run when the copy is absent or superseded.
fn previous_pin() -> Option<String> {
    let text = std::fs::read_to_string(MASK_TABLE_PATH).ok()?;
    serde_json::from_str::<MaskTable>(&text)
        .ok()
        .map(|table| table.source_commit)
}

// ── Extraction from the SystemVerilog source ──────────────────────────

/// Read every `<width>'b<bits>` literal in order, ignoring underscores.
fn binary_literals(text: &str, width: usize) -> Result<Vec<u32>, String> {
    let needle = format!("{width}'b");
    let mut values = Vec::new();
    let mut rest = text;
    while let Some(at) = rest.find(&needle) {
        let after = &rest[at + needle.len()..];
        let mut digits = String::new();
        let mut consumed = 0usize;
        for ch in after.chars() {
            match ch {
                '0' | '1' | '_' => {
                    if ch != '_' {
                        digits.push(ch);
                    }
                    consumed += ch.len_utf8();
                }
                _ => break,
            }
        }
        if digits.len() != width {
            return Err(format!(
                "a `{needle}` literal is {} bits, not {width}: `{}`",
                digits.len(),
                &after[..consumed]
            ));
        }
        values.push(
            u32::from_str_radix(&digits, 2).map_err(|e| format!("binary literal parse: {e}"))?,
        );
        rest = &after[consumed..];
    }
    Ok(values)
}

/// Identifiers following every occurrence of a literal marker, such as
/// `opcode :`.
fn identifiers_after(text: &str, marker: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(at) = rest.find(marker) {
        let after = &rest[at + marker.len()..];
        let name: String = after
            .trim_start()
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        out.push(name);
        rest = after;
    }
    out
}

/// Extract the issue-response mask table from `cvxif_instr_pkg.sv`.
///
/// The entries are read positionally: per entry the literal pairs come in
/// `instr` then `mask` order and the one-bit literals in `accept`,
/// `writeback`, `register_read[2:0]` order. Every count is asserted, and each
/// pattern must satisfy its own mask, so a reordered or mispaired entry is
/// reported rather than silently accepted.
fn extract(source: &str) -> Result<(usize, Vec<MaskEntry>), String> {
    let declaration = source
        .split("parameter int unsigned NbInstr")
        .nth(1)
        .ok_or("the source declares no `parameter int unsigned NbInstr`")?;
    let nb_instr: usize = declaration
        .split('=')
        .nth(1)
        .ok_or("`NbInstr` has no assignment")?
        .trim_start()
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>()
        .parse()
        .map_err(|e| format!("`NbInstr` is not a number: {e}"))?;

    let start = source
        .find("CoproInstr[NbInstr]")
        .ok_or("the source declares no `CoproInstr[NbInstr]`")?;
    let end = source[start..]
        .find("NbCompInstr")
        .ok_or("`CoproInstr[NbInstr]` is not followed by `NbCompInstr`")?
        + start;
    let block = &source[start..end];

    let words = binary_literals(block, INSN_WIDTH as usize)?;
    let flags = binary_literals(block, 1)?;
    let opcodes = identifiers_after(block, "opcode :");

    if words.len() != 2 * nb_instr {
        return Err(format!(
            "found {} {INSN_WIDTH}-bit literals, expected {} for {nb_instr} entries",
            words.len(),
            2 * nb_instr
        ));
    }
    if flags.len() != 5 * nb_instr {
        return Err(format!(
            "found {} one-bit literals, expected {} for {nb_instr} entries",
            flags.len(),
            5 * nb_instr
        ));
    }
    if opcodes.len() != nb_instr {
        return Err(format!(
            "found {} opcode names, expected {nb_instr}",
            opcodes.len()
        ));
    }

    let mut entries = Vec::with_capacity(nb_instr);
    for index in 0..nb_instr {
        let instr = words[2 * index];
        let mask = words[2 * index + 1];
        let accept = flags[5 * index];
        let writeback = flags[5 * index + 1];
        let register_read = [
            flags[5 * index + 2],
            flags[5 * index + 3],
            flags[5 * index + 4],
        ];
        let opcode = opcodes[index].clone();

        if accept != 1 {
            return Err(format!(
                "entry `{opcode}` has accept={accept}; a non-accepting entry would be treated as accepting"
            ));
        }
        if (mask & instr) != instr {
            return Err(format!(
                "entry `{opcode}`: pattern {instr:#010x} does not satisfy its own mask {mask:#010x}; \
                 the entry is mispaired or reordered"
            ));
        }
        entries.push(MaskEntry {
            opcode,
            instr,
            mask,
            writeback,
            register_read,
        });
    }

    Ok((nb_instr, entries))
}

// ── The checkout channel ──────────────────────────────────────────────

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

/// Header fields recovered from a checkout, used only for reporting.
struct CheckoutReport {
    commit: String,
}

/// The checkout, when one is at hand and usable, otherwise the reason it is
/// not. An unavailable checkout is a reported condition, never a failure.
fn checkout_source() -> Result<(PathBuf, CheckoutReport), String> {
    let dir = checkout_dir();
    if !dir.is_dir() {
        return Err(format!(
            "no CVA6 checkout at {}; set {CHECKOUT_ENV} to one at the pinned commit",
            dir.display()
        ));
    }
    let Some(commit) = git_head(&dir) else {
        return Err(format!(
            "{} is not a git work tree (git rev-parse HEAD failed); the revision cannot be checked",
            dir.display()
        ));
    };
    Ok((dir, CheckoutReport { commit }))
}

/// Build the committed table from a checkout.
fn table_from_checkout(dir: &Path, commit: &str) -> Result<MaskTable, String> {
    let source_path = dir.join(SOURCE_PATH);
    let source = std::fs::read_to_string(&source_path)
        .map_err(|e| format!("failed to read {}: {e}", source_path.display()))?;
    let decoder_path = dir.join(DECODER_PATH);
    let (nb_instr, entries) = extract(&source)?;
    Ok(MaskTable {
        source_path: SOURCE_PATH.to_string(),
        source_commit: commit.to_string(),
        source_sha256: sha256_file(&source_path)?,
        decoder_path: DECODER_PATH.to_string(),
        decoder_sha256: sha256_file(&decoder_path)?,
        nb_instr,
        entries,
    })
}

// ── The derivation gate ───────────────────────────────────────────────

/// One declared field's placement in the instruction word.
struct PlacedField {
    name: String,
    pos: u32,
    width: u32,
}

/// Where a table entry and a fixture disagree, or the comparison result.
struct DerivationReport {
    fixture: String,
    decision_axes: Vec<String>,
    constrained_axes: Vec<String>,
    /// Size of the enumerated decision space.
    decision_tuples: usize,
    table_tuples: BTreeSet<Vec<i64>>,
    fixture_tuples: BTreeSet<Vec<i64>>,
    /// Tuples the two models classify differently, with the table's verdict
    /// and the fixture's.
    disagreements: Vec<(Vec<i64>, bool, bool)>,
}

impl DerivationReport {
    /// The table's accepted tuples restricted to the fixture's constrained
    /// axes: the pairs the table defines in the fixture's own terms.
    fn reduced_table_tuples(&self) -> BTreeSet<Vec<i64>> {
        let indices: Vec<usize> = self
            .constrained_axes
            .iter()
            .map(|name| {
                self.decision_axes
                    .iter()
                    .position(|axis| axis == name)
                    .expect("a constrained axis is a decision axis")
            })
            .collect();
        self.table_tuples
            .iter()
            .map(|tuple| indices.iter().map(|i| tuple[*i]).collect())
            .collect()
    }
}

/// Fields the fixture's constraints reference. A value here can change
/// acceptance, so the comparison must enumerate it.
fn constrained_fields(spec: &VerificationSpec) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for constraint in &spec.constraints {
        match constraint {
            ConstraintSpec::Range { field, .. }
            | ConstraintSpec::Even { field }
            | ConstraintSpec::Lt { field, .. }
            | ConstraintSpec::Gt { field, .. }
            | ConstraintSpec::Le { field, .. }
            | ConstraintSpec::Ge { field, .. }
            | ConstraintSpec::Oneof { field, .. }
            | ConstraintSpec::Bitmask { field, .. } => {
                out.insert(field.clone());
            }
            ConstraintSpec::Eq { field_a, field_b }
            | ConstraintSpec::Neq { field_a, field_b }
            | ConstraintSpec::Cross {
                field_a, field_b, ..
            } => {
                out.insert(field_a.clone());
                out.insert(field_b.clone());
            }
            ConstraintSpec::EnableMask { field, disable, .. } => {
                out.insert(field.clone());
                out.extend(disable.iter().cloned());
            }
            ConstraintSpec::EnableSet { field, set, .. } => {
                out.insert(field.clone());
                out.extend(set.iter().map(|assignment| assignment.field.clone()));
            }
        }
    }
    out
}

/// Place every declared field in the word, checking the domain fits the bits
/// and that no two fields overlap.
fn place_fields(spec: &VerificationSpec, fixture: &str) -> Result<Vec<PlacedField>, String> {
    let layout = spec.encoding.as_ref().ok_or_else(|| {
        format!("{fixture}: the spec declares no encoding layout, so fields carry no bit positions")
    })?;
    let mut placed: Vec<PlacedField> = Vec::new();
    for (name, field) in spec.fields.iter() {
        let mapping = layout.field_map.get(name).ok_or_else(|| {
            format!("{fixture}: field `{name}` has no bit mapping in the encoding layout")
        })?;
        if mapping.width == 0 || mapping.width >= INSN_WIDTH {
            return Err(format!(
                "{fixture}: field `{name}` has width {}, which cannot sit in a {INSN_WIDTH}-bit word",
                mapping.width
            ));
        }
        if mapping.pos + mapping.width > INSN_WIDTH {
            return Err(format!(
                "{fixture}: field `{name}` at position {} width {} exceeds the {INSN_WIDTH}-bit word",
                mapping.pos, mapping.width
            ));
        }
        let domain = field.expand();
        let (Some(min), Some(max)) = (domain.first(), domain.iter().max()) else {
            return Err(format!("{fixture}: field `{name}` has an empty domain"));
        };
        if *min < 0 || (*max as u64) >> mapping.width != 0 {
            return Err(format!(
                "{fixture}: field `{name}` domain reaches {max}, which does not fit its {} bits",
                mapping.width
            ));
        }
        placed.push(PlacedField {
            name: name.clone(),
            pos: mapping.pos,
            width: mapping.width,
        });
    }
    for (i, a) in placed.iter().enumerate() {
        for b in placed.iter().skip(i + 1) {
            let overlaps = a.pos < b.pos + b.width && b.pos < a.pos + a.width;
            if overlaps {
                return Err(format!(
                    "{fixture}: fields `{}` and `{}` overlap in the instruction word",
                    a.name, b.name
                ));
            }
        }
    }
    Ok(placed)
}

fn field_covering(placed: &[PlacedField], bit: u32) -> Option<&PlacedField> {
    placed
        .iter()
        .find(|field| bit >= field.pos && bit < field.pos + field.width)
}

/// Pack a decision tuple into an instruction word. Bits outside the decision
/// axes stay zero; the precondition guarantees no entry masks them.
fn pack_word(placed: &[PlacedField], tuple: &[i64]) -> u32 {
    let mut word = 0u32;
    for (field, value) in placed.iter().zip(tuple) {
        word |= (*value as u32) << field.pos;
    }
    word
}

fn table_accepts(word: u32, entries: &[MaskEntry]) -> bool {
    entries
        .iter()
        .any(|entry| (entry.mask & word) == entry.instr)
}

/// The fixture's accepted set over the decision axes, computed by the
/// production pipeline on a spec restricted to those axes.
fn fixture_accepted(spec: &VerificationSpec, axes: &[String]) -> Vec<(Vec<i64>, bool)> {
    let mut fields: BTreeMap<String, FieldSpec> = BTreeMap::new();
    let mut layout = EncodingLayout {
        insn_width: spec
            .encoding
            .as_ref()
            .map(|l| l.insn_width)
            .unwrap_or(INSN_WIDTH),
        field_map: BTreeMap::new(),
    };
    for name in axes {
        fields.insert(name.clone(), spec.fields[name].clone());
        if let Some(mapping) = spec.encoding.as_ref().and_then(|l| l.field_map.get(name)) {
            layout.field_map.insert(name.clone(), *mapping);
        }
    }
    // The projector is irrelevant to acceptance, and a projector naming a
    // field the axes exclude would not resolve; Sum always resolves.
    let reduced = VerificationSpec {
        target: spec.target.clone(),
        fields,
        encoding: Some(layout),
        constraints: spec.constraints.clone(),
        projector: ProjectorSpec::Sum,
    };

    let combinations = expand_all(&reduced).expect("the decision space must expand");
    evaluate_all(
        &reduced,
        combinations,
        &ConstraintRegistry::default(),
        &ProjectorRegistry::default(),
    )
    .into_iter()
    .map(|evaluation| (evaluation.values, evaluation.passed))
    .collect()
}

/// Compare one derived fixture with the table.
fn check_fixture(path: &str, table: &MaskTable) -> Result<DerivationReport, String> {
    let spec = VerificationSpec::from_yaml(Path::new(path))
        .map_err(|e| format!("{path}: cannot load the fixture: {e}"))?;
    let placed = place_fields(&spec, path)?;

    // Precondition: every bit an entry masks must be covered by a declared
    // field, or the projection onto those fields would drop it.
    for entry in &table.entries {
        for bit in 0..INSN_WIDTH {
            if entry.mask >> bit & 1 == 1 && field_covering(&placed, bit).is_none() {
                return Err(format!(
                    "{path}: entry `{}` masks bit {bit}, which no declared field covers; \
                     the projection onto the fixture's fields would lose it",
                    entry.opcode
                ));
            }
        }
    }

    // Decision axes: every field an entry masks, plus every field the
    // fixture's constraints reference.
    let mut axes: BTreeSet<String> = constrained_fields(&spec);
    for entry in &table.entries {
        for bit in 0..INSN_WIDTH {
            if entry.mask >> bit & 1 == 1 {
                if let Some(field) = field_covering(&placed, bit) {
                    axes.insert(field.name.clone());
                }
            }
        }
    }

    // Restrict the placement to the decision axes, in the axes' order.
    let axis_names: Vec<String> = axes.iter().cloned().collect();
    let axis_placed: Vec<PlacedField> = axis_names
        .iter()
        .map(|name| {
            let field = placed
                .iter()
                .find(|field| &field.name == name)
                .expect("a decision axis is a declared field");
            PlacedField {
                name: field.name.clone(),
                pos: field.pos,
                width: field.width,
            }
        })
        .collect();

    let mut table_tuples = BTreeSet::new();
    let mut fixture_tuples = BTreeSet::new();
    let mut disagreements = Vec::new();
    let mut decision_tuples = 0usize;
    for (tuple, passed) in fixture_accepted(&spec, &axis_names) {
        decision_tuples += 1;
        let accepted = table_accepts(pack_word(&axis_placed, &tuple), &table.entries);
        if accepted {
            table_tuples.insert(tuple.clone());
        }
        if passed {
            fixture_tuples.insert(tuple.clone());
        }
        if accepted != passed {
            disagreements.push((tuple, accepted, passed));
        }
    }

    Ok(DerivationReport {
        fixture: path.to_string(),
        decision_axes: axes.iter().cloned().collect(),
        constrained_axes: constrained_fields(&spec).iter().cloned().collect(),
        decision_tuples,
        table_tuples,
        fixture_tuples,
        disagreements,
    })
}

fn render(fixture: &str, axes: &[String], tuples: &BTreeSet<Vec<i64>>, limit: usize) -> String {
    let mut lines: Vec<String> = tuples
        .iter()
        .take(limit)
        .map(|tuple| {
            let pairs: Vec<String> = axes
                .iter()
                .zip(tuple)
                .map(|(name, value)| format!("{name}={value}"))
                .collect();
            format!("    {{{}}}", pairs.join(", "))
        })
        .collect();
    if tuples.len() > limit {
        lines.push(format!("    ... {} more", tuples.len() - limit));
    }
    if lines.is_empty() {
        lines.push("    (none)".to_string());
    }
    let suffix = if tuples.len() == 1 { "" } else { "s" };
    format!(
        "    {} tuple{suffix} for {fixture}\n{}",
        tuples.len(),
        lines.join("\n")
    )
}

// ── Tests ─────────────────────────────────────────────────────────────

/// Channel 1: every derived fixture matches the table.
#[test]
fn fixtures_match_the_mask_table() {
    let table = load_committed_table();
    let mut failures = Vec::new();
    for fixture in DERIVED_FIXTURES {
        match check_fixture(fixture, &table) {
            Err(message) => failures.push(message),
            Ok(report) => {
                eprintln!(
                    "{}: decision axes {:?}, {} table tuples, {} fixture tuples",
                    report.fixture,
                    report.decision_axes,
                    report.table_tuples.len(),
                    report.fixture_tuples.len()
                );
                eprintln!(
                    "{}",
                    render(
                        &report.fixture,
                        &report.constrained_axes,
                        &report.reduced_table_tuples(),
                        32
                    )
                );
                if !report.disagreements.is_empty() {
                    let detail: Vec<String> = report
                        .disagreements
                        .iter()
                        .take(8)
                        .map(|(tuple, accepted, passed)| {
                            let pairs: Vec<String> = report
                                .decision_axes
                                .iter()
                                .zip(tuple)
                                .map(|(name, value)| format!("{name}={value}"))
                                .collect();
                            format!(
                                "    {{{}}}: table {}, fixture {}",
                                pairs.join(", "),
                                if *accepted { "accepts" } else { "rejects" },
                                if *passed { "accepts" } else { "rejects" }
                            )
                        })
                        .collect();
                    failures.push(format!(
                        "{}: {} of {} decision tuples disagree:\n{}\n  table accepts {} tuples, fixture accepts {}",
                        report.fixture,
                        report.disagreements.len(),
                        report.decision_tuples,
                        detail.join("\n"),
                        report.table_tuples.len(),
                        report.fixture_tuples.len()
                    ));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "the derived fixtures disagree with the committed mask table:\n{}",
        failures.join("\n")
    );
}

/// The acceptance criterion for `xif_ref`: the table defines exactly the six
/// custom-3 `(funct3, funct7)` pairs, and the fixture accepts exactly those.
#[test]
fn xif_ref_reports_the_six_decoder_pairs() {
    let table = load_committed_table();
    let report = check_fixture("tests/fixtures/cva6/xif_ref.xif.yaml", &table)
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        report.constrained_axes,
        vec!["funct3".to_string(), "funct7".to_string()],
        "the fixture's constrained axes"
    );
    let expected: BTreeSet<Vec<i64>> = [(0, 0), (1, 0), (1, 1), (1, 2), (1, 3), (1, 4)]
        .into_iter()
        .map(|(funct3, funct7)| vec![funct3, funct7])
        .collect();
    assert_eq!(
        report.reduced_table_tuples(),
        expected,
        "the table must define the six custom-3 (funct3, funct7) pairs"
    );
    assert!(
        report.disagreements.is_empty(),
        "the fixture must accept exactly the table's pairs"
    );
}

/// Channel 2: when a checkout is at hand, the committed extraction must match
/// it, and the checkout must be at the pinned commit.
#[test]
fn source_mask_table_matches_the_checkout() {
    let table = load_committed_table();
    let (dir, report) = match checkout_source() {
        Ok(found) => found,
        Err(reason) => {
            eprintln!("{reason}; the source channel did not run");
            return;
        }
    };
    eprintln!("CVA6 checkout {} at {}", dir.display(), report.commit);
    assert_eq!(
        report.commit,
        table.source_commit,
        "the checkout at {} is at {}, but the committed extraction was taken at {}",
        dir.display(),
        report.commit,
        table.source_commit
    );
    let digest = sha256_file(&dir.join(&table.source_path)).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        digest, table.source_sha256,
        "{} has digest {digest}, but the committed extraction records {}",
        table.source_path, table.source_sha256
    );
    let decoder_digest =
        sha256_file(&dir.join(&table.decoder_path)).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        decoder_digest, table.decoder_sha256,
        "{} has digest {decoder_digest}, but the committed extraction records {}",
        table.decoder_path, table.decoder_sha256
    );
    let extracted = table_from_checkout(&dir, &report.commit).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        extracted, table,
        "the re-extracted table differs from the committed copy at {MASK_TABLE_PATH}"
    );
}

/// Regeneration: rewrite the committed extraction from the checkout. Opt-in,
/// so an ordinary test run never mutates the working tree.
#[test]
fn update_mask_table_when_requested() {
    if std::env::var(UPDATE_ENV).as_deref() != Ok("1") {
        eprintln!("{UPDATE_ENV} is not 1; the regeneration path did not run");
        return;
    }
    let (dir, report) = checkout_source().unwrap_or_else(|reason| panic!("{reason}"));
    if let Some(pinned) = previous_pin() {
        if report.commit != pinned {
            eprintln!(
                "warning: writing the extraction from {} at {}, previously pinned to {pinned}",
                dir.display(),
                report.commit,
            );
        }
    }
    let table = table_from_checkout(&dir, &report.commit).unwrap_or_else(|e| panic!("{e}"));
    let mut text = serde_json::to_string_pretty(&table).expect("the table must serialize");
    text.push('\n');
    std::fs::write(MASK_TABLE_PATH, text)
        .unwrap_or_else(|e| panic!("failed to write {MASK_TABLE_PATH}: {e}"));
    eprintln!(
        "wrote {}: {} entries from {} at {}",
        MASK_TABLE_PATH,
        table.entries.len(),
        table.source_path,
        table.source_commit
    );
}
