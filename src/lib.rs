//! ev — ExaVerif: exhaustive verification of a declared discrete space.
//!
//! This library crate provides the core verification pipeline: field spec
//! parsing, combinatorial domain expansion, constraint evaluation, projection,
//! and output formatting. A target is whatever its spec declares: a field, a
//! domain, and the constraints between fields. Nothing in the engine names an
//! instruction set or a core.
//!
//! Target families enter through two seams:
//!
//! - [`profile::LayoutRegistry`] supplies named bit layouts, so a spec can
//!   select `riscv-r` (with the `riscv` feature) or a layout registered by the
//!   caller.
//! - [`format::FormatCapable`] parses the spec, and a caller can supply its
//!   own format or its own registry.
//!
//! # Architecture
//!
//! ```text
//! lib.rs          ← public API re-exports
//! spec/           ← VerificationSpec, FieldSpec, ConstraintSpec, ProjectorSpec, WordLayout
//! profile/        ← LayoutRegistry; riscv.rs holds the RISC-V layouts (feature = "riscv")
//! classification/ ← Classification, Verdict: the engine's output type
//! verify/         ← compose, evaluate, registry (Check trait + registries)
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
