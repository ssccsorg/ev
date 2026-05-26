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

        /// Explain failures in natural language via LLM
        #[arg(long)]
        interpret: bool,

        /// Run external synthesis after verification
        #[arg(long)]
        synth: bool,
    },
    /// Generate a signed verification certificate
    Certify {
        /// Path to the YAML constraint file
        #[arg(short, long)]
        target: PathBuf,

        /// Output path for the certificate
        #[arg(short, long)]
        output: Option<String>,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Check {
            target,
            json,
            interpret,
            synth,
        } => {
            if interpret {
                anyhow::bail!("--interpret is not yet implemented");
            }

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
                all_passed = reporter.report(&spec.target, &field_order, &evaluations);
            }

            // Run synthesis alongside verification when requested.
            if synth {
                let report = synth::synthesize_default(&spec)?;
                if json {
                    let fact: fih::Fact = report.into();
                    println!("{}", serde_json::to_string_pretty(&fact)?);
                } else {
                    println!("Synthesis: {}", report.module_name);
                    println!("  backend:  {}", report.tool);
                    println!("  version:  {}", report.version);
                    println!("  gate count: {:?}", report.gate_count);
                    println!("  cell area:  {:?}", report.cell_area);
                }
            }

            if !all_passed {
                std::process::exit(1);
            }
        }
        Commands::Certify { target, output } => {
            let path = output.unwrap_or_else(|| "certificate.pdf".to_string());
            println!("ev certify --target {} --output {}", target.display(), path);
            anyhow::bail!("certify is not yet implemented");
        }
    }

    Ok(())
}
