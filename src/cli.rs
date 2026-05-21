use std::path::PathBuf;

use clap::Parser;

use crate::encoding::Encoding;

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

    /// Console character encoding (DOS bytes → terminal characters)
    #[arg(long, value_enum, default_value_t = Encoding::Gbk)]
    pub encoding: Encoding,

    /// Path to a virtual 1.44MB floppy image for int 13h disk reads/writes.
    #[arg(long)]
    pub disk: Option<PathBuf>,

    /// Run to completion in headless mode (no TUI) and print final CPU state.
    /// Developer-only switch used to validate VM behaviour from the command line.
    #[arg(long, hide = true)]
    pub run: bool,

    /// Maximum number of instructions to execute when `--run` is set.
    #[arg(long, default_value_t = 1_000_000)]
    pub max_steps: u64,
}
