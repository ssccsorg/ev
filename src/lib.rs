//! ev — ExaVerif: exhaustive verification for RISC-V custom instructions.
//!
//! This library crate provides the core verification pipeline: field spec parsing,
//! combinatorial domain expansion, constraint evaluation, projection, and output
//! formatting. It contains **no** external tool coupling.
//!
//! # Architecture
//!
//! ```text
//! lib.rs         ← public API re-exports
//! spec/          ← VerificationSpec, FieldSpec, ConstraintSpec, ProjectorSpec
//! verify/        ← compose, evaluate, registry (Check trait + registries)
//! report/        ← ReporterCapable trait + Fact/Intent/Hint + implementations
//! format/        ← FormatCapable trait + XIF format parser
//! synth/         ← synthesis traits (GenerateRtl, RunSynthesis), SV generation, backends
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

pub mod format;
pub mod report;
pub mod spec;
pub mod synth;
pub mod verify;
