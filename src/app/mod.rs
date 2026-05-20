pub mod editor;
pub mod event;
pub mod keymap;

use std::collections::HashSet;
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
use crate::vm::i8086::exec::{StepOutcome, Vm};
use crate::vm::i8086::memory::Memory;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunStatus {
    Paused,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallFrame {
    pub return_cs: u16,
    pub return_ip: u16,
    pub from_line: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptKind {
    Goto,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prompt {
    pub kind: PromptKind,
    pub label: &'static str,
    pub buffer: String,
}

pub const SOURCE_PANE_TITLE: &str = "Source";
pub const CONSOLE_PANE_TITLE: &str = "Console";
pub const REGISTERS_PANE_TITLE: &str = "Registers";
pub const MEMORY_PANE_TITLE: &str = "Memory";

pub struct App {
    file: PathBuf,
    source_text: String,
    program: Program, // reset 用底版
    vm: Option<Vm>,
    status: RunStatus,
    steps_executed: u64,
    focus: FocusPane,
    source_scroll: u16,
    source_cursor: u32, // 1-based 源码行
    memory_origin_seg: u16,
    memory_origin_off: u16,
    console_input_buf: Vec<u8>,
    console_scroll: u16,
    breakpoints: HashSet<u32>, // 物理地址
    call_stack: Vec<CallFrame>,
    prompt: Option<Prompt>,
    editor_pending: bool, // 主循环看到 true 后调起外部编辑器并清屏
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
            program: program.clone(),
            vm: None,
            status: RunStatus::Paused,
            steps_executed: 0,
            focus: FocusPane::Source,
            source_scroll: 0,
            source_cursor: 1,
            memory_origin_seg: 0,
            memory_origin_off: 0,
            console_input_buf: Vec::new(),
            console_scroll: 0,
            breakpoints: HashSet::new(),
            call_stack: Vec::new(),
            prompt: None,
            editor_pending: false,
            should_quit: false,
            max_steps,
            mem_kb,
        };
        app.reboot_vm(program);
        app
    }

    /// 用给定 program 重新 boot VM，但保留断点。停在入口（Paused）。
    fn reboot_vm(&mut self, program: Program) {
        self.steps_executed = 0;
        self.console_input_buf.clear();
        self.console_scroll = 0;
        self.call_stack.clear();
        match Vm::boot(program, self.mem_kb) {
            Ok(vm) => {
                self.memory_origin_seg = vm.cpu.ds;
                self.memory_origin_off = 0;
                self.vm = Some(vm);
                self.status = RunStatus::Paused;
                self.source_cursor = self.highlighted_line().unwrap_or(1);
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
        self.program = program.clone();
        // reload 时**清空**断点（源码变了，物理地址不再可信）
        self.breakpoints.clear();
        self.reboot_vm(program);
        Ok(())
    }

    /// 复位：保留断点和文件，重新 boot VM。
    pub fn reset(&mut self) {
        let program = self.program.clone();
        self.reboot_vm(program);
    }

    /// 单步：执行一条指令。维护 call_stack 与 status。
    pub fn step_once(&mut self) {
        if !matches!(self.status, RunStatus::Paused) {
            return;
        }
        let Some(vm) = self.vm.as_mut() else { return };

        // 抓当前 slot 用于 call/ret 跟踪
        let slot_info = vm.current_slot().map(|s| {
            (
                s.instr.mnemonic.clone(),
                s.span.line,
                s.ip_offset.wrapping_add(s.size),
                vm.cpu.cs,
            )
        });

        match vm.step() {
            Ok(StepOutcome::Stepped) => {
                self.steps_executed += 1;
                if let Some((mn, line, ret_ip, ret_cs)) = slot_info {
                    match mn.as_str() {
                        "call" => self.call_stack.push(CallFrame {
                            return_cs: ret_cs,
                            return_ip: ret_ip,
                            from_line: Some(line),
                        }),
                        "ret" | "retf" => {
                            self.call_stack.pop();
                        }
                        _ => {}
                    }
                }
            }
            Ok(StepOutcome::Halted) => {
                self.status = RunStatus::Halted;
            }
            Err(e) => {
                self.status = RunStatus::Error(e.to_string());
            }
        }
        // 同步 cursor 到当前 PC
        if let Some(line) = self.highlighted_line() {
            self.source_cursor = line;
        }
    }

    /// 持续单步直到 halt / error / 命中断点 / 超过 max_steps。
    pub fn run_continue(&mut self) {
        if !matches!(self.status, RunStatus::Paused) {
            return;
        }
        let limit = self.max_steps.saturating_sub(self.steps_executed);
        let mut budget = limit.min(1_000_000);
        loop {
            self.step_once();
            if !matches!(self.status, RunStatus::Paused) {
                return;
            }
            // 命中断点？（当前 ip 已经指向下一条要执行的指令）
            if let Some(phys) = self.current_phys()
                && self.breakpoints.contains(&phys)
            {
                return;
            }
            if budget == 0 {
                self.status = RunStatus::Error(format!(
                    "执行超过 {} 步未停机（防卡保护，按 c 继续）",
                    self.max_steps
                ));
                return;
            }
            budget -= 1;
        }
    }

    /// 步过：若当前指令是 call，则跑到 call 之后那条；否则等价于单步。
    pub fn step_over(&mut self) {
        if !matches!(self.status, RunStatus::Paused) {
            return;
        }
        let Some(vm) = self.vm.as_ref() else { return };
        let target = match vm.current_slot() {
            Some(slot) if slot.instr.mnemonic == "call" => Some(Memory::phys(
                vm.cpu.cs,
                slot.ip_offset.wrapping_add(slot.size),
            )),
            _ => None,
        };
        let Some(target_phys) = target else {
            self.step_once();
            return;
        };

        let mut budget = self.max_steps.min(1_000_000);
        loop {
            self.step_once();
            if !matches!(self.status, RunStatus::Paused) {
                return;
            }
            if Some(target_phys) == self.current_phys() {
                return;
            }
            // 命中断点也停
            if let Some(phys) = self.current_phys()
                && self.breakpoints.contains(&phys)
            {
                return;
            }
            if budget == 0 {
                return;
            }
            budget -= 1;
        }
    }

    fn current_phys(&self) -> Option<u32> {
        let vm = self.vm.as_ref()?;
        Some(Memory::phys(vm.cpu.cs, vm.cpu.ip))
    }

    pub fn breakpoints(&self) -> &HashSet<u32> {
        &self.breakpoints
    }

    /// 该源码行是否有断点（任意段任意指令）。
    pub fn line_has_breakpoint(&self, line: u32) -> bool {
        if let Some(vm) = self.vm.as_ref() {
            for seg in vm.program.segments.values() {
                for slot in &seg.instructions {
                    if slot.span.line == line
                        && self
                            .breakpoints
                            .contains(&Memory::phys(seg.base_paragraph, slot.ip_offset))
                    {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// 在 source_cursor 当前行 toggle 断点。
    pub fn toggle_breakpoint_at_cursor(&mut self) {
        let Some(vm) = self.vm.as_ref() else { return };
        let line = self.source_cursor;
        // 找第一个 span.line == cursor 的 InstrSlot
        let mut target: Option<u32> = None;
        for seg in vm.program.segments.values() {
            for slot in &seg.instructions {
                if slot.span.line == line {
                    target = Some(Memory::phys(seg.base_paragraph, slot.ip_offset));
                    break;
                }
            }
            if target.is_some() {
                break;
            }
        }
        if let Some(phys) = target
            && !self.breakpoints.remove(&phys)
        {
            self.breakpoints.insert(phys);
        }
    }

    pub fn call_stack(&self) -> &[CallFrame] {
        &self.call_stack
    }

    pub fn source_cursor(&self) -> u32 {
        self.source_cursor
    }

    pub fn move_cursor(&mut self, delta: i32) {
        let total = self.source_text.lines().count() as u32;
        if total == 0 {
            return;
        }
        let cur = self.source_cursor as i32 + delta;
        let new = cur.clamp(1, total as i32) as u32;
        self.source_cursor = new;
        // 滚屏跟上：粗略让 cursor 始终在可视区内（具体可视高度未知，留 3 行余地）
        if new < self.source_scroll as u32 + 1 {
            self.source_scroll = new.saturating_sub(1) as u16;
        } else if new > self.source_scroll as u32 + 20 {
            self.source_scroll = new.saturating_sub(20) as u16;
        }
    }

    pub fn cursor_to_line(&mut self, line: u32) {
        let total = self.source_text.lines().count() as u32;
        if total == 0 {
            return;
        }
        self.source_cursor = line.clamp(1, total);
    }

    pub fn prompt(&self) -> Option<&Prompt> {
        self.prompt.as_ref()
    }

    pub fn open_prompt(&mut self, kind: PromptKind) {
        let label = match kind {
            PromptKind::Goto => "Goto seg:off / 标签 / 物理地址",
        };
        self.prompt = Some(Prompt {
            kind,
            label,
            buffer: String::new(),
        });
    }

    pub fn close_prompt(&mut self) {
        self.prompt = None;
    }

    pub fn prompt_push(&mut self, c: char) {
        if let Some(p) = self.prompt.as_mut() {
            p.buffer.push(c);
        }
    }

    pub fn prompt_backspace(&mut self) {
        if let Some(p) = self.prompt.as_mut() {
            p.buffer.pop();
        }
    }

    /// 提交 prompt：按 kind 解释 buffer 并执行。
    pub fn prompt_submit(&mut self) {
        let Some(prompt) = self.prompt.take() else {
            return;
        };
        match prompt.kind {
            PromptKind::Goto => self.apply_goto(&prompt.buffer),
        }
    }

    fn apply_goto(&mut self, raw: &str) {
        let Some(vm) = self.vm.as_mut() else { return };
        let s = raw.trim();
        // 1) "seg:off" 形式（hex 或 dec）
        if let Some((seg_part, off_part)) = s.split_once(':')
            && let (Ok(seg), Ok(off)) = (parse_word(seg_part), parse_word(off_part))
        {
            vm.cpu.cs = seg;
            vm.set_ip(off);
            if let Some(line) = self.highlighted_line() {
                self.source_cursor = line;
            }
            return;
        }
        // 2) 标签名
        if let Some(sym) = vm.program.symbols.get(s) {
            let seg = vm.program.segments[&sym.segment].base_paragraph;
            vm.cpu.cs = seg;
            vm.set_ip(sym.offset);
            if let Some(line) = self.highlighted_line() {
                self.source_cursor = line;
            }
            return;
        }
        // 3) 单一物理地址 hex
        if let Ok(phys) = parse_phys(s) {
            let seg = (phys >> 4) as u16;
            let off = (phys & 0xF) as u16;
            vm.cpu.cs = seg;
            vm.set_ip(off);
            if let Some(line) = self.highlighted_line() {
                self.source_cursor = line;
            }
            return;
        }
        // 解析失败：用 Error 状态展示
        self.status = RunStatus::Error(format!("无法解析 goto 目标 `{raw}`"));
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

    pub fn set_memory_origin(&mut self, seg: u16, off: u16) {
        self.memory_origin_seg = seg;
        self.memory_origin_off = off & !0x000F;
    }

    pub fn scroll_memory(&mut self, delta_bytes: i32) {
        let cur = self.memory_origin_off as i32;
        let next = (cur + delta_bytes).max(0).min(u16::MAX as i32) as u16;
        self.memory_origin_off = next & !0x000F;
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

    pub fn request_editor(&mut self) {
        self.editor_pending = true;
    }

    pub fn take_editor_request(&mut self) -> bool {
        std::mem::take(&mut self.editor_pending)
    }

    pub fn highlighted_line(&self) -> Option<u32> {
        let vm = self.vm.as_ref()?;
        let seg = vm
            .program
            .segments
            .values()
            .find(|s| s.base_paragraph == vm.cpu.cs)?;
        // Halted 之后 ip 已指向"下一条"，可能正好落在物理上紧邻的别的指令上，
        // 此时不应把 ▶ 标到那条不会执行的指令；强制回退到最后已执行的那条。
        if !matches!(self.status, RunStatus::Halted)
            && let Some(slot) = seg.instructions.iter().find(|s| s.ip_offset == vm.cpu.ip)
        {
            return Some(slot.span.line);
        }
        seg.instructions
            .iter()
            .filter(|s| s.ip_offset < vm.cpu.ip)
            .max_by_key(|s| s.ip_offset)
            .map(|s| s.span.line)
    }
}

fn parse_word(s: &str) -> std::result::Result<u16, ()> {
    let s = s.trim();
    let s_lower = s.to_ascii_lowercase();
    if let Some(rest) = s_lower.strip_suffix('h') {
        return u16::from_str_radix(rest, 16).map_err(|_| ());
    }
    if let Some(rest) = s_lower.strip_prefix("0x") {
        return u16::from_str_radix(rest, 16).map_err(|_| ());
    }
    // 默认按 hex（debug.exe 习惯）
    u16::from_str_radix(s, 16).map_err(|_| ())
}

fn parse_phys(s: &str) -> std::result::Result<u32, ()> {
    let s = s.trim();
    let s_lower = s.to_ascii_lowercase();
    if let Some(rest) = s_lower.strip_suffix('h') {
        return u32::from_str_radix(rest, 16).map_err(|_| ());
    }
    if let Some(rest) = s_lower.strip_prefix("0x") {
        return u32::from_str_radix(rest, 16).map_err(|_| ());
    }
    u32::from_str_radix(s, 16).map_err(|_| ())
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
        if app.take_editor_request() {
            let file = app.file().clone();
            editor::launch_editor(&file)?;
            // 强制下一次 draw 是完整重绘（编辑器扰乱了底层 buffer）
            terminal.clear()?;
            if let Err(err) = app.reload() {
                tracing::warn!("reload failed: {err}");
            }
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
