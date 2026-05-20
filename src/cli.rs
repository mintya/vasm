use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "vasm", about = "Visualize 8086 assembly in your terminal")]
pub struct Cli {
    /// Path to a .asm source file
    pub file: PathBuf,

    /// Write logs to this file (no logging if omitted)
    #[arg(long)]
    pub log: Option<PathBuf>,

    /// Simulated memory size in KiB (max 1024)
    #[arg(long, default_value_t = 1024)]
    pub mem_kb: u32,
}
