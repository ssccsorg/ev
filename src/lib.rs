//! ev — ExaVerif: exhaustive verification of a declared discrete space.
//!
//! This library crate provides the core verification pipeline: field spec
//! parsing, combinatorial domain expansion, constraint evaluation, projection,
//! and output formatting. A target is whatever its spec declares: a field, a
//! domain, and the constraints between fields. Nothing in the engine names an
//! instruction set or a core.
//!
//! Target families enter through three seams:
//!
//! - [`profile::LayoutRegistry`] supplies named bit layouts, so a spec can
//!   select a built-in profile or a layout registered by the caller.
//! - [`format::FormatCapable`] parses the spec, and a caller can supply its
//!   own format or its own registry.
//! - [`verify::Classify`] lets an engine replace the built-in structural
//!   enumerator without changing the record.
//!
//! # Layer discipline
//!
//! The engine is always present. A target module can contribute two layers:
//!
//! - a layout profile, which gives a spec a word to pack its fields into;
//! - a simulator backend, which executes a spec's points on an external tool.
//!
//! `targets` is the aggregate that selects the target modules for the default
//! build, and RISC-V is its only member today. The pure generic core is
//! `--no-default-features`; `--no-default-features --features riscv` selects
//! one module explicitly. The CLI exposes the same seam with
//! `ev capabilities`.
//!
//! # Architecture
//!
//! ```text
//! lib.rs          ← public API re-exports and the layer discipline
//! spec/           ← VerificationSpec, FieldSpec, ConstraintSpec, ProjectorSpec, WordLayout
//! profile/        ← LayoutRegistry; the generic core has no entries
//! classification/ ← Classification, Verdict: the engine's output type
//! verify/         ← compose, evaluate, classifier (Classify trait + engines), registry
//! report/         ← ReporterCapable trait + Fact envelope + implementations
//! format/         ← FormatCapable trait + XIF format parser
//! synth/          ← synthesis traits (GenerateRtl, RunSynthesis), SV generation, backends
//! ```
//!
//! # Consuming as a library
//!
//! ```rust,ignore
//! use ev::spec::VerificationSpec;
//! use ev::verify::{expand_all, evaluate_all, ConstraintRegistry, ProjectorRegistry};
//!
//! let spec = VerificationSpec::from_yaml(&std::path::Path::new("spec.xif.yaml")).unwrap();
//! let combos = expand_all(&spec).unwrap();
//! let results = evaluate_all(&spec, combos, &ConstraintRegistry::default(), &ProjectorRegistry::default());
//! ```

pub mod classification;
pub mod format;
pub mod profile;
pub mod report;
pub mod spec;
pub mod synth;
pub mod verify;
