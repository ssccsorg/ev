mod compose;
mod evaluate;
mod fih;
mod format;
mod registry;
mod reporter;
mod spec;
mod synth;
mod xif;

use clap::{Parser, Subcommand, ValueEnum};
use registry::ConstraintRegistry;
use registry::ProjectorRegistry;
use reporter::{JsonReporter, ReporterCapable, TextReporter, CsvReporter, TraceReporter};
use std::path::PathBuf;
use synth::backends::yosys::YosysBackend;
use synth::sim::{MockSimBackend, RunSimulation, SimulationResult};
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

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
enum OutputFormat {
    Text,
    Json,
    Csv,
    Trace,
}

#[derive(Subcommand)]
enum Commands {
    /// Static constraint verification against field specification
    Verify {
        /// Path to the YAML constraint file (XIF format)
        #[arg(short, long)]
        target: PathBuf,

        /// Output results as JSON instead of text (deprecated, use --format instead)
        #[arg(long)]
        json: bool,

        /// Output format: text, json, csv, or trace (default: text)
        #[arg(long, value_enum)]
        format: Option<OutputFormat>,
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

    /// ISA simulation verification via Spike
    Simulate {
        /// Path to the YAML constraint file
        #[arg(short, long)]
        target: PathBuf,

        /// Output results as JSON instead of text (deprecated, use --format instead)
        #[arg(long)]
        json: bool,

        /// Output format: text, json, csv, or trace (default: text)
        #[arg(long, value_enum)]
        format: Option<OutputFormat>,
    },

    /// Decode a Fact envelope from stdin
    Fact {
        #[command(subcommand)]
        command: FactCommands,
    },
}

#[derive(Subcommand)]
enum FactCommands {
    /// Decode a Fact JSON from stdin and print the payload as plain text
    Decode,
}

fn resolve_sim_backend() -> Box<dyn RunSimulation> {
    match std::env::var("EV_SIM_BACKEND").unwrap_or_default().as_str() {
        "spike" => Box::new(synth::backends::spike::SpikeBackend),
        _ => Box::new(MockSimBackend),
    }
}

fn resolve_synth_backend() -> Box<dyn RunSynthesis> {
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

fn run_sim(target: &std::path::Path) -> anyhow::Result<SimulationResult> {
    use compose::expand_all;
    use evaluate::evaluate_all;
    let spec = spec::VerificationSpec::from_yaml(target)?;
    let constraint_registry = ConstraintRegistry::default();
    let projector_registry = ProjectorRegistry::default();
    let combinations =
        expand_all(&spec).map_err(|e| anyhow::anyhow!("domain expansion failed: {}", e))?;
    let evaluations = evaluate_all(
        &spec,
        combinations,
        &constraint_registry,
        &projector_registry,
    );
    let backend = resolve_sim_backend();
    backend.run(&spec, evaluations)
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
        Commands::Verify { target, json, format } => {
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

            let fmt = format.unwrap_or(if json { OutputFormat::Json } else { OutputFormat::Text });
            let reporter: Box<dyn ReporterCapable> = match fmt {
                OutputFormat::Json => Box::new(JsonReporter),
                OutputFormat::Csv => Box::new(CsvReporter),
                OutputFormat::Trace => Box::new(TraceReporter),
                OutputFormat::Text => Box::new(TextReporter),
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
        Commands::Simulate { target, json, format } => {
            let result = run_sim(&target)?;
            let n = result.evaluations.len();
            let passed = result.evaluations.iter().filter(|e| e.passed).count();
            let failed = n - passed;
            let fmt = format.unwrap_or(if json { OutputFormat::Json } else { OutputFormat::Text });
            match fmt {
                OutputFormat::Json => {
                    let fact: fih::Fact = (&result).into();
                    println!("{}", serde_json::to_string_pretty(&fact).unwrap());
                }
                OutputFormat::Csv => {
                    let reporter = CsvReporter;
                    reporter.report(&result.tool, "", &[], &result.evaluations);
                }
                OutputFormat::Trace => {
                    let reporter = TraceReporter;
                    reporter.report(&result.tool, "", &[], &result.evaluations);
                }
                OutputFormat::Text => {
                    println!("target: simulation ({} backend)", result.tool);
                    println!("total:  {}", n);
                    println!("passed: {}", passed);
                    println!("failed: {}", failed);
                }
            }
            if failed > 0 {
                std::process::exit(1);
            }
        }
        Commands::Fact { command } => match command {
            FactCommands::Decode => {
                let mut input = String::new();
                std::io::Read::read_to_string(&mut std::io::stdin(), &mut input)
                    .map_err(|e| anyhow::anyhow!("failed to read stdin: {}", e))?;
                let fact: fih::Fact = serde_json::from_str(&input)
                    .map_err(|e| anyhow::anyhow!("failed to parse Fact JSON: {}", e))?;
                // Try UTF-8 decode first, then fall back to hex
                match String::from_utf8(fact.payload.clone()) {
                    Ok(text) => print!("{}", text),
                    Err(_) => {
                        println!("payload (hex): {}", hex::encode(&fact.payload));
                    }
                }
            }
        },
    }

    Ok(())
}
