use std::fs::OpenOptions;
use std::sync::Mutex;

use clap::Parser;
use tracing_subscriber::EnvFilter;

use vasm::asm::diagnostics::Severity;
use vasm::cli::Cli;
use vasm::error::Result;

fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Some(log_path) = &cli.log {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)?;
        tracing_subscriber::fmt()
            .with_writer(Mutex::new(file))
            .with_env_filter(
                EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
            )
            .init();
    }

    let (program, diags) = vasm::asm::parse_file(&cli.file)?;
    let file_label = cli.file.display().to_string();
    let mut has_error = false;
    for d in &diags {
        eprintln!("{}", d.format(&file_label));
        if d.severity == Severity::Error {
            has_error = true;
        }
    }
    if has_error {
        std::process::exit(1);
    }

    vasm::app::run(cli, program)
}
