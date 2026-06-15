//! Core simulation abstractions — traits and shared types.
//!
//! Tool-agnostic layer for ISA simulation backends. Any RISC-V simulator
//! (Spike, QEMU, riscvOVPsim, custom ISS) can be plugged in by implementing
//! the `RunSimulation` trait. The core layer knows nothing about any specific
//! tool.
//!
//! # Architecture
//!
//! ```text
//! RunSimulation        ← spec + encodings → evaluations (trait)
//!  ├── MockSimBackend  ← test/CI: static passthrough
//!  └── SpikeBackend    ← Spike ISA simulator
//!       (future) QEMUBackend     ← QEMU system-mode emulation
//!       (future) OVPsimBackend   ← riscvOVPsim/CORE-V simulation
//! ```
//!
//! # Capability-based design (mirrors Nexus storage traits)
//!
//! Instead of a monolithic simulator interface, functionality is split into
//! fine-grained capability traits. Each backend implements only what it supports.
//!
//! ```text
//! SimulateBackend       — base: run simulation on encodings
//!  ├── BatchCapable     — run multiple encodings in a single invocation
//!  ├── ProfileCapable   — return performance metrics (cycles, IPC)
//!  └── CoverageCapable  — return coverage data (which encodings exercised)
//! ```

use crate::evaluate::Evaluation;
use crate::spec::VerificationSpec;

// ============================================================================
// Simulation traits
// ============================================================================

/// Result of a single simulation run.
///
/// Every simulation backend produces the same result type. Backend-specific
/// data (e.g. Spike version, QEMU machine config) goes into `extra` as an
/// opaque JSON value.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SimulationResult {
    /// Tool identifier: "spike", "qemu", "mock", etc.
    pub tool: String,
    /// Tool version string.
    pub version: String,
    /// Ordered field names matching evaluation values.
    pub field_order: Vec<String>,
    /// Evaluations after simulation — each encoding marked pass/fail.
    /// Same structure as `evaluate::evaluate_all` output.
    pub evaluations: Vec<Evaluation>,
    /// Backend-specific opaque data (opaque to core).
    pub extra: Option<serde_json::Value>,
}

/// Base capability: run a simulator on a spec's encodings.
///
/// The backend receives the full spec (for field names, constraints) and
/// the pre-computed evaluations from static verification. It must return
/// evaluations with simulation results merged.
///
/// # Error contract
///
/// * `Ok(result)` — simulation ran to completion. Tool-level failures
///   (encoding rejected, crash) are encoded in individual Evaluation records.
/// * `Err(...)` — infrastructure failure: tool not found, cross-compiler
///   missing, ELF generation failed.
pub trait RunSimulation: Send + Sync {
    /// Run simulation on all valid encodings from a verification spec.
    ///
    /// `spec` — the full specification (field definitions, constraints).
    /// `static_evaluations` — pre-computed evaluations from `evaluate_all`.
    ///
    /// Returns evaluations with Spike results merged. Encodings that passed
    /// static verification but failed simulation are marked as failed.
    fn run(
        &self,
        spec: &VerificationSpec,
        static_evaluations: Vec<Evaluation>,
    ) -> anyhow::Result<SimulationResult>;
}

// ============================================================================
// Optional capability traits (future)
// ============================================================================

#[allow(dead_code)]
/// Backend can run multiple encoding batches in a single invocation.
pub trait BatchCapable: RunSimulation {
    /// Maximum encodings per batch.
    fn batch_size(&self) -> usize;
}

#[allow(dead_code)]
/// Backend can return performance profiling data.
pub trait ProfileCapable: RunSimulation {
    fn cycles(&self) -> Option<u64>;
    fn ipc(&self) -> Option<f64>;
}

#[allow(dead_code)]
/// Backend can return coverage data.
pub trait CoverageCapable: RunSimulation {
    fn coverage_map(&self) -> Option<serde_json::Value>;
}

// ============================================================================
// MockSimBackend — test/CI backend (no external tool required)
// ============================================================================

/// Test backend that passes all valid encodings through without running a
/// real simulator. Used in CI and unit tests where Spike is unavailable.
///
/// Behaviour: all encodings that passed static verification are reported
/// as passed. No tool is invoked.
impl From<&SimulationResult> for crate::fih::Fact {
    fn from(r: &SimulationResult) -> Self {
        let origin = format!("ev/simulation/{}", r.tool);
        let payload = serde_json::json!({
            "tool": r.tool,
            "version": r.version,
            "total": r.evaluations.len(),
            "passed": r.evaluations.iter().filter(|e| e.passed).count(),
            "failed": r.evaluations.iter().filter(|e| !e.passed).count(),
        });
        crate::fih::Fact::new(
            "simulation_result",
            &origin,
            "simulation",
            serde_json::to_vec(&payload).unwrap_or_default(),
        )
    }
}

pub struct MockSimBackend;

impl RunSimulation for MockSimBackend {
    fn run(
        &self,
        spec: &VerificationSpec,
        static_evaluations: Vec<Evaluation>,
    ) -> anyhow::Result<SimulationResult> {
        let field_order: Vec<String> = spec.fields.keys().cloned().collect();
        Ok(SimulationResult {
            tool: "mock".into(),
            version: "0.0.0".into(),
            field_order,
            evaluations: static_evaluations,
            extra: None,
        })
    }
}
