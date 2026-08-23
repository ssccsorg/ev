pub mod compose;
pub mod evaluate;
pub mod registry;

pub use compose::{expand_all, raw_total_combinations};
pub use evaluate::{evaluate_all, evaluate_structural};
pub use registry::{ConstraintRegistry, ProjectorRegistry};
