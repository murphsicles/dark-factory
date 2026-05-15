// Dark Factory — Rust → Zeta transpiler
// Autonomous crate conversion pipeline.
// (c) 2026 Zeta Foundation. MIT licensed.

use clap::{Parser, Subcommand};

mod transpiler;
mod rewrites;
mod pipeline;
mod zeta;
mod post_process;

#[derive(Parser)]
#[command(name = "df", about = "Rust → Zeta transpiler", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Convert a single Rust source file to Zeta
    Convert {
        /// Path to input .rs file
        input: String,
        /// Output path (default: stdout)
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Convert an entire crate directory
    Crate {
        /// Path to the crate directory (must have Cargo.toml)
        path: String,
    },
    /// Fetch a crate from crates.io and convert it
    Fetch {
        /// Crate name
        name: String,
        /// Version (default: latest)
        #[arg(short, long)]
        version: Option<String>,
    },
    /// Run the full pipeline on a crate
    Pipeline {
        /// Crate name
        name: String,
        /// Version (default: latest)
        #[arg(short, long)]
        version: Option<String>,
        /// Publish to zorbs.io after successful conversion
        #[arg(short, long)]
        publish: bool,
    },
    /// List known rewrite rules
    Rules,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Convert { input, output } => {
            let source = std::fs::read_to_string(&input)?;
            let result = transpiler::convert_file(&source, &input)?;
            match output {
                Some(path) => std::fs::write(&path, &result)?,
                None => println!("{}", result),
            }
        }
        Command::Crate { path } => {
            pipeline::convert_crate(&path)?;
        }
        Command::Fetch { name, version } => {
            let version = version.unwrap_or_else(|| "*".to_string());
            pipeline::fetch_and_convert(&name, &version)?;
        }
        Command::Pipeline { name, version, publish } => {
            let version = version.unwrap_or_else(|| "*".to_string());
            pipeline::run_pipeline(&name, &version, publish)?;
        }
        Command::Rules => {
            println!("Available rewrite rules:");
            for rule in rewrites::list_rules() {
                println!("  {}", rule);
            }
        }
    }
    Ok(())
}
