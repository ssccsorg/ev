mod compose;
mod evaluate;
mod fih;
mod format;
mod registry;
mod reporter;
mod spec;
mod synth;
mod xif;

use clap::{Parser, Subcommand};
use registry::ConstraintRegistry;
use registry::ProjectorRegistry;
use reporter::{JsonReporter, ReporterCapable, TextReporter};
use std::path::PathBuf;
use synth::backends::yosys::YosysBackend;
use synth::{GenerateRtl, MockSynthesisBackend, RunSynthesis, SvGenerator};

#[derive(Parser)]
#[command(
    name = "ev",
    version,
    about = "Exhaustive verification for custom instruction extensions"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Verify an instruction against its field specification
    Check {
        /// Path to the YAML constraint file (XIF format)
        #[arg(short, long)]
        target: PathBuf,

        /// Output results as JSON instead of text
        #[arg(long)]
        json: bool,

        /// Run external synthesis after verification
        #[arg(long)]
        synth: bool,
    },
}

fn resolve_synth_backend() -> Box<dyn RunSynthesis> {
    // Policy decision: environment variables control backend selection.
    // This is the only place where ev chooses a backend — the library
    // layer does not know about env vars or CLI flags.
    if std::env::var("EV_SYNTH_BACKEND").unwrap_or_default() == "mock" {
        return Box::new(MockSynthesisBackend);
    }
    Box::new(YosysBackend)
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Check {
            target,
            json,
            synth,
        } => {
            // Resolve input format by extension.
            let spec = spec::VerificationSpec::from_yaml(&target)?;

            // Default registries — extensible via plugin system in future phases.
            let constraint_registry = ConstraintRegistry::default();
            let projector_registry = ProjectorRegistry::default();

            let combinations = compose::expand_all(&spec);
            let evaluations = evaluate::evaluate_all(
                &spec,
                combinations,
                &constraint_registry,
                &projector_registry,
            );

            // Always report verification results first.
            let all_passed;
            {
                let reporter: Box<dyn ReporterCapable> = if json {
                    Box::new(JsonReporter)
                } else {
                    Box::new(TextReporter)
                };

                let field_order: Vec<String> = spec.fields.keys().cloned().collect();
                let spec_hash = reporter::hash_spec(&spec);
                all_passed = reporter.report(&spec.target, &spec_hash, &field_order, &evaluations);
            }

            // Run synthesis alongside verification when requested.
            if synth {
                let backend = resolve_synth_backend();
                let rtl_path = SvGenerator.generate(&spec)?;
                let report = backend.run(&rtl_path, &spec.target)?;

                if json {
                    let fact: fih::Fact = report.into();
                    println!("{}", serde_json::to_string_pretty(&fact)?);
                } else {
                    let status_label = if report.status == "ok" {
                        "ok"
                    } else {
                        "FAILED"
                    };
                    println!("Synthesis: {} [{}]", report.module_name, status_label);
                    println!("  backend:  {}", report.tool);
                    println!("  version:  {}", report.version);
                    println!("  gate count: {:?}", report.gate_count);
                    println!("  cell area:  {:?}", report.cell_area);
                    if let Some(ref msg) = report.message {
                        if report.status != "ok" {
                            println!("  error:     {}", msg.trim());
                        }
                    }
                }
            }

            if !all_passed {
                std::process::exit(1);
            }
        }
    }

    Ok(())
}
