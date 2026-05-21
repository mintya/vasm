use std::fs::OpenOptions;
use std::sync::Mutex;

use clap::Parser;
use tracing_subscriber::EnvFilter;

use vasm::asm::diagnostics::Severity;
use vasm::cli::Cli;
use vasm::error::Result;
use vasm::vm::i8086::exec::Vm;

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

    if cli.run {
        let disk = load_disk(&cli)?;
        let mut vm =
            Vm::boot_with_disk(program, cli.mem_kb, disk).map_err(|e| anyhow::anyhow!("{e}"))?;
        vm.run_until_halt(cli.max_steps)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        print_state(&vm);
        if !vm.console.output().is_empty() {
            println!("--- console output ---");
            println!("{}", cli.encoding.decode(vm.console.output()));
        }
        return Ok(());
    }

    let disk = load_disk(&cli)?;
    vasm::app::run(cli, program, disk)
}

fn load_disk(cli: &Cli) -> Result<Option<Vec<u8>>> {
    match &cli.disk {
        Some(path) => Ok(Some(std::fs::read(path)?)),
        None => Ok(None),
    }
}

fn print_state(vm: &Vm) {
    let c = &vm.cpu;
    println!("=== VM halted ===");
    println!(
        "ax={:04X} bx={:04X} cx={:04X} dx={:04X}",
        c.ax, c.bx, c.cx, c.dx
    );
    println!(
        "si={:04X} di={:04X} bp={:04X} sp={:04X}",
        c.si, c.di, c.bp, c.sp
    );
    println!(
        "cs={:04X} ds={:04X} ss={:04X} es={:04X}  ip={:04X}",
        c.cs, c.ds, c.ss, c.es, c.ip
    );
    println!(
        "flags: CF={} PF={} AF={} ZF={} SF={} OF={}",
        c.flags.cf as u8,
        c.flags.pf as u8,
        c.flags.af as u8,
        c.flags.zf as u8,
        c.flags.sf as u8,
        c.flags.of as u8,
    );
}
