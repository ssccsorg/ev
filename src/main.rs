mod compose;
mod evaluate;
mod report;
mod xif;

use clap::{Parser, Subcommand};
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
        } => {
            if interpret {
                anyhow::bail!("--interpret is not yet implemented");
            }

            let doc = xif::XifDocument::from_path(&target)?;
            let combinations = compose::expand_all(&doc);
            let evaluations = evaluate::evaluate_all(&doc, combinations);

            let all_passed = if json {
                let names: Vec<&str> = doc.field_names();
                report::report_json(&doc.target, &names, &evaluations)
            } else {
                report::report_text(&doc.target, &evaluations)
            };

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
