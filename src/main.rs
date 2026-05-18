use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "ev", version, about = "Exhaustive verification for RISC-V custom instructions")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Verify a module against its constraint specification
    Check {
        /// Path to the YAML constraint file
        #[arg(short, long)]
        target: String,

        /// Explain failures in natural language via LLM
        #[arg(long)]
        interpret: bool,
    },
    /// Generate a signed verification certificate
    Certify {
        /// Path to the YAML constraint file
        #[arg(short, long)]
        target: String,

        /// Output path for the certificate PDF
        #[arg(short, long)]
        output: Option<String>,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Check { target, interpret } => {
            println!("ev check --target {target}");
            if interpret {
                println!("  (--interpret enabled)");
            }
        }
        Commands::Certify { target, output } => {
            let path = output.unwrap_or_else(|| "certificate.pdf".to_string());
            println!("ev certify --target {target} --output {path}");
        }
    }

    Ok(())
}
