pub mod editor;
pub mod event;
pub mod keymap;

use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::Duration;

use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use crate::asm::ast::Program;
use crate::asm::diagnostics::Severity;
use crate::asm::parser;
use crate::cli::Cli;
use crate::error::Result;
use crate::ui;
use crate::vm::i8086::exec::{StepOutcome, Vm, VmError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunStatus {
    Halted,
    Error(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusPane {
    Source,
    Console,
    Registers,
    Memory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Control,
    Input,
}

pub const SOURCE_PANE_TITLE: &str = "Source";
pub const CONSOLE_PANE_TITLE: &str = "Console";
pub const REGISTERS_PANE_TITLE: &str = "Registers";
pub const MEMORY_PANE_TITLE: &str = "Memory";

pub struct App {
    file: PathBuf,
    source_text: String,
    vm: Option<Vm>,
    status: RunStatus,
    steps_executed: u64,
    focus: FocusPane,
    source_scroll: u16,
    memory_origin_seg: u16,
    memory_origin_off: u16,
    console_input_buf: Vec<u8>,
    console_scroll: u16,
    should_quit: bool,
    max_steps: u64,
    mem_kb: u32,
}

impl App {
    pub fn boot(
        file: PathBuf,
        source_text: String,
        program: Program,
        mem_kb: u32,
        max_steps: u64,
    ) -> Self {
        let mut app = Self {
            file,
            source_text,
            vm: None,
            status: RunStatus::Halted,
            steps_executed: 0,
            focus: FocusPane::Source,
            source_scroll: 0,
            memory_origin_seg: 0,
            memory_origin_off: 0,
            console_input_buf: Vec::new(),
            console_scroll: 0,
            should_quit: false,
            max_steps,
            mem_kb,
        };
        app.boot_vm(program);
        app
    }

    fn boot_vm(&mut self, program: Program) {
        self.steps_executed = 0;
        self.console_input_buf.clear();
        match Vm::boot(program, self.mem_kb) {
            Ok(mut vm) => {
                // 单步推到 halt，方便 steps 计数；超过 max_steps 视为错误。
                let mut hit_error: Option<VmError> = None;
                for _ in 0..self.max_steps {
                    match vm.step() {
                        Ok(StepOutcome::Stepped) => self.steps_executed += 1,
                        Ok(StepOutcome::Halted) => break,
                        Err(e) => {
                            hit_error = Some(e);
                            break;
                        }
                    }
                }
                if !vm.halted() && hit_error.is_none() {
                    hit_error = Some(VmError::StepLimitExceeded {
                        max_steps: self.max_steps,
                    });
                }
                // 默认 memory 起点：ds:0
                self.memory_origin_seg = vm.cpu.ds;
                self.memory_origin_off = 0;
                self.status = match hit_error {
                    Some(e) => RunStatus::Error(e.to_string()),
                    None => RunStatus::Halted,
                };
                self.vm = Some(vm);
            }
            Err(e) => {
                self.vm = None;
                self.status = RunStatus::Error(e.to_string());
            }
        }
    }

    pub fn reload(&mut self) -> io::Result<()> {
        let source = fs::read_to_string(&self.file)?;
        let (program, diags) = parser::parse(&source);
        // 把诊断作为错误显示在状态栏（汇总成一行）
        let parse_error = diags
            .iter()
            .find(|d| d.severity == Severity::Error)
            .map(|d| d.format(&self.file.display().to_string()));
        self.source_text = source;
        if let Some(msg) = parse_error {
            self.vm = None;
            self.status = RunStatus::Error(msg);
            self.steps_executed = 0;
            return Ok(());
        }
        self.boot_vm(program);
        Ok(())
    }

    pub fn file(&self) -> &PathBuf {
        &self.file
    }

    pub fn file_display(&self) -> String {
        self.file
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.file.to_string_lossy().into_owned())
    }

    pub fn source_text(&self) -> &str {
        &self.source_text
    }

    pub fn vm(&self) -> Option<&Vm> {
        self.vm.as_ref()
    }

    pub fn status(&self) -> &RunStatus {
        &self.status
    }

    pub fn steps_executed(&self) -> u64 {
        self.steps_executed
    }

    pub fn focus(&self) -> FocusPane {
        self.focus
    }

    pub fn mode(&self) -> InputMode {
        match self.focus {
            FocusPane::Console => InputMode::Input,
            _ => InputMode::Control,
        }
    }

    pub fn set_focus(&mut self, focus: FocusPane) {
        self.focus = focus;
    }

    pub fn cycle_focus(&mut self, forward: bool) {
        const RING: [FocusPane; 4] = [
            FocusPane::Source,
            FocusPane::Console,
            FocusPane::Registers,
            FocusPane::Memory,
        ];
        let idx = RING.iter().position(|f| *f == self.focus).unwrap_or(0);
        let next = if forward {
            (idx + 1) % RING.len()
        } else {
            (idx + RING.len() - 1) % RING.len()
        };
        self.focus = RING[next];
    }

    pub fn source_scroll(&self) -> u16 {
        self.source_scroll
    }

    pub fn scroll_source(&mut self, delta: i32) {
        let v = self.source_scroll as i32 + delta;
        self.source_scroll = v.max(0) as u16;
    }

    pub fn memory_origin(&self) -> (u16, u16) {
        (self.memory_origin_seg, self.memory_origin_off)
    }

    pub fn scroll_memory(&mut self, delta_bytes: i32) {
        let cur = self.memory_origin_off as i32;
        let next = (cur + delta_bytes).max(0).min(u16::MAX as i32) as u16;
        self.memory_origin_off = next & !0x000F; // 按 16 字节对齐
    }

    pub fn console_input(&self) -> &[u8] {
        &self.console_input_buf
    }

    pub fn push_console_input(&mut self, b: u8) {
        self.console_input_buf.push(b);
    }

    pub fn console_scroll(&self) -> u16 {
        self.console_scroll
    }

    pub fn scroll_console(&mut self, delta: i32) {
        let v = self.console_scroll as i32 + delta;
        self.console_scroll = v.max(0) as u16;
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    /// 返回当前 cs:ip 对应的源码行（1-based）。
    /// 若 ip 落在某条指令上则返回该指令的行；否则返回最大 ip_offset < cpu.ip 的指令的行。
    pub fn highlighted_line(&self) -> Option<u32> {
        let vm = self.vm.as_ref()?;
        let seg = vm
            .program
            .segments
            .values()
            .find(|s| s.base_paragraph == vm.cpu.cs)?;
        if let Some(slot) = seg.instructions.iter().find(|s| s.ip_offset == vm.cpu.ip) {
            return Some(slot.span.line);
        }
        seg.instructions
            .iter()
            .filter(|s| s.ip_offset < vm.cpu.ip)
            .max_by_key(|s| s.ip_offset)
            .map(|s| s.span.line)
    }
}

struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> Result<Self> {
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen)?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }
}

pub fn run(cli: Cli, program: Program) -> Result<()> {
    install_panic_hook();

    let source_text = fs::read_to_string(&cli.file)?;
    let mut app = App::boot(
        cli.file.clone(),
        source_text,
        program,
        cli.mem_kb,
        cli.max_steps,
    );

    let _guard = TerminalGuard::enter()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    while !app.should_quit {
        terminal.draw(|f| ui::render(f, &app))?;
        if let Some(key) = event::poll_key(Duration::from_millis(100))? {
            keymap::handle(key, &mut app)?;
        }
    }

    Ok(())
}

fn install_panic_hook() {
    let original = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        original(info);
    }));
}
