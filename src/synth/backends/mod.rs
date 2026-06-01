//! External synthesis tool backends.
//!
//! Each file in this module implements `RunSynthesis` for a specific tool.
//! Only the files here know about external CLIs. Adding a new tool means
//! adding a new file here — nothing else changes.

pub mod spike;
pub mod yosys;
