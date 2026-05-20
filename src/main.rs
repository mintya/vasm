mod app;
mod asm;
mod cli;
mod error;
mod trace;
mod ui;
mod vm;

use std::fs::OpenOptions;
use std::sync::Mutex;

use clap::Parser;
use tracing_subscriber::EnvFilter;

use crate::error::Result;

fn main() -> Result<()> {
    let cli = cli::Cli::parse();

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

    app::run(cli)
}
