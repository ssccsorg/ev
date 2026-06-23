//! ev — ExaVerif: exhaustive verification for RISC-V custom instructions.

pub mod spec;
pub mod verify {
    pub mod compose;
    pub mod evaluate;
    pub mod registry;
    pub use compose::expand_all;
    pub use evaluate::evaluate_all;
    pub use registry::{ConstraintRegistry, ProjectorRegistry};
}
pub mod report {
    pub mod fih;
    pub mod reporter;
    pub use fih::Fact;
    pub use reporter::{
        CsvReporter, JsonReporter, ReporterCapable, TextReporter, TraceReporter, hash_spec,
    };
}
pub mod format;
pub mod synth;
