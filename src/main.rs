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
    /// Static constraint verification against field specification
    Verify {
        /// Path to the YAML constraint file (XIF format)
        #[arg(short, long)]
        target: PathBuf,

        /// Output results as JSON instead of text
        #[arg(long)]
        json: bool,
    },

    /// SystemVerilog RTL generation and synthesis
    Synth {
        /// Path to the YAML constraint file
        #[arg(short, long)]
        target: PathBuf,

        /// Output results as JSON instead of text
        #[arg(long)]
        json: bool,
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

fn print_synthesis_report(report: &synth::SynthesisMetrics, json: bool) {
    if json {
        let fact: fih::Fact = report.clone().into();
        println!("{}", serde_json::to_string_pretty(&fact).unwrap());
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

fn run_synth(target: &std::path::Path) -> anyhow::Result<synth::SynthesisMetrics> {
    let spec = spec::VerificationSpec::from_yaml(target)?;
    let rtl_path = SvGenerator.generate(&spec)?;
    let backend = resolve_synth_backend();
    backend.run(&rtl_path, &spec.target)
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Verify { target, json } => {
            let spec = spec::VerificationSpec::from_yaml(&target)?;

            let constraint_registry = ConstraintRegistry::default();
            let projector_registry = ProjectorRegistry::default();

            let combinations = compose::expand_all(&spec)
                .map_err(|e| anyhow::anyhow!("domain expansion failed: {}", e))?;
            let evaluations = evaluate::evaluate_all(
                &spec,
                combinations,
                &constraint_registry,
                &projector_registry,
            );

            let reporter: Box<dyn ReporterCapable> = if json {
                Box::new(JsonReporter)
            } else {
                Box::new(TextReporter)
            };

            let field_order: Vec<String> = spec.fields.keys().cloned().collect();
            let spec_hash = reporter::hash_spec(&spec);
            let all_passed = reporter.report(&spec.target, &spec_hash, &field_order, &evaluations);

            if !all_passed {
                std::process::exit(1);
            }
        }
        Commands::Synth { target, json } => {
            let report = run_synth(&target)?;
            print_synthesis_report(&report, json);
            if !report.status.eq_ignore_ascii_case("ok") {
                anyhow::bail!("synthesis failed: {}", report.message.unwrap_or_default());
            }
        }
    }

    Ok(())
}
