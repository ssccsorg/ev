//! ev — ExaVerif: exhaustive verification for RISC-V custom instructions.
//!
//! This library crate provides the core verification pipeline: spec parsing,
//! combinatorial field expansion, constraint evaluation, projection, and
//! synthesis trait abstractions. It contains **no** external tool coupling.
//!
//! # Architecture
//!
//! ```text
//! lib.rs              ← public API re-exports
//! spec.rs             ← VerificationSpec, FieldSpec, ConstraintSpec, ProjectorSpec
//! compose.rs          ← combinatorial field expansion
//! evaluate.rs         ← constraint + projector evaluation
//! registry.rs         ← constraint/projector lookup
//! reporter.rs         ← ReporterCapable trait + TextReporter + JsonReporter
//! fih.rs              ← Fact/Intent/Hint primitives
//! format.rs           ← input format parsing
//! xif.rs              ← XIF format support
//! synth/mod.rs        ← synthesis traits (GenerateRtl, RunSynthesis, FullSynthesis),
//!                        SynthesisMetrics, SvGenerator, generate_sv, MockSynthesisBackend
//! synth/backends/     ← external tool backends (YosysBackend, ...)
//! ```
//!
//! # Consuming as a library
//!
//! ```rust,ignore
//! // This example requires a real .xif.yaml file to run.
//! use ev::spec::VerificationSpec;
//! use ev::synth::{GenerateRtl, RunSynthesis, SvGenerator, MockSynthesisBackend};
//!
//! let spec = VerificationSpec::from_yaml(&std::path::Path::new("spec.xif.yaml")).unwrap();
//! let rtl_path = SvGenerator.generate(&spec).unwrap();
//! let metrics = MockSynthesisBackend.run(&rtl_path, &spec.target).unwrap();
//! ```

pub mod compose;

pub mod evaluate;
pub mod fih;
pub mod format;
pub mod registry;
pub mod reporter;
pub mod spec;
pub mod synth;
pub mod xif;
