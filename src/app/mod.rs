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
use crate::encoding::Encoding;
use crate::error::Result;
use crate::ui;
use crate::vm::i8086::exec::{StepOutcome, StepSnapshot, Vm};
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

/// 用户敲在 Console 焦点下的一个字符的视觉副本。`bytes` 是它在 `vm.console.input`
/// 字节流里占用的字节数——程序消费时按这个数从 echo 头部弹掉对应字符。
/// `display` 是渲染时使用的字符串（控制字符用 caret notation 如 `^H` `^M` `^I`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EchoChar {
    pub display: String,
    pub bytes: u8,
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
    AddWatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prompt {
    pub kind: PromptKind,
    pub label: &'static str,
    pub buffer: String,
}

/// Watchpoint：寄存器或物理地址 word。值变化时自动 Paused。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Watch {
    Reg(String),
    Mem(u32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatchEntry {
    pub spec: Watch,
    pub last_value: u16,
}

pub const SOURCE_PANE_TITLE: &str = "Source";
pub const CONSOLE_PANE_TITLE: &str = "Console";
pub const REGISTERS_PANE_TITLE: &str = "Registers";
pub const MEMORY_PANE_TITLE: &str = "Memory";

/// 历史栈上限，防 OOM。
const HISTORY_CAP: usize = 1024;

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
    console_scroll: u16,
    /// 用户在 Console 焦点下敲入字符的视觉副本（已编码字节数 + 显示用字符）。
    /// 每帧 render 前调 `sync_echo`，按 vm.console.input 当前长度从头弹掉
    /// 已被程序消费的字符——所以未消费的部分始终可见，消费过的会消失（并由
    /// 程序自己的 output 接手呈现）。
    console_echo: Vec<EchoChar>,
    /// 执行历史快照栈，每次 step 入栈一个。`u` 弹一个回退；`U` 弹到上个断点。
    /// 上限 HISTORY_CAP，溢出时丢掉最旧的（FIFO 截断）。
    history: Vec<StepSnapshot>,
    /// 观察点列表。每次 step 后比对，命中变化即 Paused。
    watches: Vec<WatchEntry>,
    /// 最近一次命中的 watch（用于状态栏展示）。
    last_watch_hit: Option<String>,
    breakpoints: HashSet<u32>, // 物理地址
    call_stack: Vec<CallFrame>,
    prompt: Option<Prompt>,
    editor_pending: bool, // 主循环看到 true 后调起外部编辑器并清屏
    should_quit: bool,
    max_steps: u64,
    mem_kb: u32,
    encoding: Encoding,
    disk: Option<Vec<u8>>,
}

impl App {
    pub fn boot(
        file: PathBuf,
        source_text: String,
        program: Program,
        mem_kb: u32,
        max_steps: u64,
        encoding: Encoding,
        disk: Option<Vec<u8>>,
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
            console_scroll: 0,
            console_echo: Vec::new(),
            history: Vec::new(),
            watches: Vec::new(),
            last_watch_hit: None,
            breakpoints: HashSet::new(),
            call_stack: Vec::new(),
            prompt: None,
            editor_pending: false,
            should_quit: false,
            max_steps,
            mem_kb,
            encoding,
            disk,
        };
        app.reboot_vm(program);
        app
    }

    /// 用给定 program 重新 boot VM，但保留断点。停在入口（Paused）。
    fn reboot_vm(&mut self, program: Program) {
        self.steps_executed = 0;
        self.console_scroll = 0;
        self.console_echo.clear();
        self.history.clear();
        self.call_stack.clear();
        match Vm::boot_with_disk(program, self.mem_kb, self.disk.clone()) {
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

    /// 单步：执行一条指令。维护 call_stack 与 status，并把 snapshot 入历史栈。
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

        match vm.step_with_snapshot() {
            Ok((StepOutcome::Stepped, snap)) => {
                self.push_history(snap);
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
                // 检查观察点；命中则状态保持 Paused（已经是 Paused）并记录命中描述
                if let Some(desc) = self.check_watches() {
                    self.last_watch_hit = Some(desc);
                }
            }
            Ok((StepOutcome::Halted, snap)) => {
                self.push_history(snap);
                self.status = RunStatus::Halted;
            }
            Ok((StepOutcome::WaitingForInput, _snap)) => {
                // WaitingForInput 的 snapshot 不入历史——下次 step 会重新执行同一条指令，
                // 入栈会让 undo 行为难以推断。
                self.focus = FocusPane::Console;
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

    fn push_history(&mut self, snap: StepSnapshot) {
        if self.history.len() >= HISTORY_CAP {
            self.history.remove(0); // FIFO 截断最旧
        }
        self.history.push(snap);
    }

    /// 比对每个 watch 的当前值与 last_value。任意变化即返回描述串（"ax 0001→0042"）
    /// 并更新 last_value；无变化返 None。
    fn check_watches(&mut self) -> Option<String> {
        let vm = self.vm.as_ref()?;
        let mut hit_desc: Option<String> = None;
        for w in self.watches.iter_mut() {
            let cur = match &w.spec {
                Watch::Reg(name) => read_reg_value(vm, name).unwrap_or(0),
                Watch::Mem(addr) => vm.mem.read_u16(*addr).unwrap_or(0),
            };
            if cur != w.last_value {
                let label = match &w.spec {
                    Watch::Reg(n) => n.clone(),
                    Watch::Mem(a) => format!("[{a:05X}]"),
                };
                hit_desc = Some(format!("{label} {:04X}→{:04X}", w.last_value, cur));
                w.last_value = cur;
            }
        }
        hit_desc
    }

    pub fn watches(&self) -> &[WatchEntry] {
        &self.watches
    }

    pub fn last_watch_hit(&self) -> Option<&str> {
        self.last_watch_hit.as_deref()
    }

    /// 添加观察点。`raw` 可为寄存器名（ax/bx/...）或 seg:off / 物理地址。
    pub fn add_watch(&mut self, raw: &str) -> std::result::Result<(), String> {
        let s = raw.trim().to_ascii_lowercase();
        let Some(vm) = self.vm.as_ref() else {
            return Err("vm 未启动".into());
        };
        // 寄存器
        if let Some(val) = read_reg_value(vm, &s) {
            self.watches.push(WatchEntry {
                spec: Watch::Reg(s),
                last_value: val,
            });
            return Ok(());
        }
        // seg:off
        if let Some((seg_part, off_part)) = s.split_once(':')
            && let (Ok(seg), Ok(off)) = (parse_word(seg_part), parse_word(off_part))
        {
            let addr = Memory::phys(seg, off);
            let val = vm.mem.read_u16(addr).map_err(|e| e.to_string())?;
            self.watches.push(WatchEntry {
                spec: Watch::Mem(addr),
                last_value: val,
            });
            return Ok(());
        }
        // 物理地址 hex
        if let Ok(addr) = parse_phys(&s) {
            let val = vm.mem.read_u16(addr).map_err(|e| e.to_string())?;
            self.watches.push(WatchEntry {
                spec: Watch::Mem(addr),
                last_value: val,
            });
            return Ok(());
        }
        Err(format!("无法解析 watch `{raw}`"))
    }

    pub fn clear_watches(&mut self) {
        self.watches.clear();
        self.last_watch_hit = None;
    }

    /// undo：弹一个 snapshot，按字段把 vm 状态还原到 step 之前。
    pub fn undo(&mut self) {
        let Some(snap) = self.history.pop() else {
            return;
        };
        let Some(vm) = self.vm.as_mut() else { return };
        // 1. 倒序回放 mem diffs（写回旧值）
        for (addr, old) in snap.mem_diffs.iter().rev() {
            let _ = vm.mem.write_u8(*addr, *old);
        }
        // 2. 还原 cpu
        vm.cpu = snap.cpu_before;
        // 3. 还原 console
        vm.console.truncate_output(snap.console_output_len_before);
        vm.console.restore_input(snap.console_input_before);
        vm.console.set_waiting(false);
        // 4. 还原 halted
        if !snap.halted_before {
            // 用 hack：halted 是私有字段，没有 unset_halted；用 reset 不动 vm
            // 但 vm 的 halted_before=false 意味着回退后应"可继续 step"。
            // Vm 没暴露 setter——加一个？这里通过 vm.halt() 反向：
            // 由于现有 API 没有 unhalt，这里依赖 halted 已是私有，需要新增公开接口。
            // 暂时假设 undo 跨过 halted 边界少见，留 TODO；当前实现：若需要 unhalt，
            // App 通过强制重新设 vm.cpu.ip 间接：无效。
            // 真正做法：给 Vm 加 set_halted。
            vm.set_halted(false);
        } else {
            vm.set_halted(true);
        }
        // 5. App 同步
        self.steps_executed = self.steps_executed.saturating_sub(1);
        self.status = RunStatus::Paused;
        // call_stack 不准确但教学场景能接受——准确还原需要把 call_stack 也存进 snapshot
        if let Some(line) = self.highlighted_line() {
            self.source_cursor = line;
        }
    }

    /// undo 到最近的断点位置（含当前 ip 即为断点的情况），或回到入口。
    pub fn undo_to_breakpoint(&mut self) {
        // 至少 undo 一步避免原地停留
        let mut first = true;
        loop {
            if self.history.is_empty() {
                return;
            }
            self.undo();
            if first {
                first = false;
                continue;
            }
            if let Some(phys) = self.current_phys()
                && self.breakpoints.contains(&phys)
            {
                return;
            }
        }
    }

    /// 持续单步直到 halt / error / 命中断点 / 等待输入 / 超过 max_steps。
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
            // 等待输入：停下让用户敲键
            if self
                .vm
                .as_ref()
                .map(|vm| vm.console.waiting_for_input())
                .unwrap_or(false)
            {
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
            if self
                .vm
                .as_ref()
                .map(|vm| vm.console.waiting_for_input())
                .unwrap_or(false)
            {
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
            PromptKind::AddWatch => "Watch 寄存器 / seg:off / 物理地址",
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

    /// 关闭诊断浮层：Error 状态切回 Paused，VM 状态保留以便用户继续检查。
    pub fn dismiss_error(&mut self) {
        if matches!(self.status, RunStatus::Error(_)) {
            self.status = RunStatus::Paused;
        }
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
            PromptKind::AddWatch => {
                if let Err(msg) = self.add_watch(&prompt.buffer) {
                    self.status = RunStatus::Error(msg);
                }
            }
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

    pub fn vm_mut(&mut self) -> Option<&mut Vm> {
        self.vm.as_mut()
    }

    pub fn encoding(&self) -> Encoding {
        self.encoding
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

    pub fn console_scroll(&self) -> u16 {
        self.console_scroll
    }

    /// 当前 echo 视觉副本（已自动同步过——render 前应先调 `sync_echo`）。
    pub fn console_echo(&self) -> &[EchoChar] {
        &self.console_echo
    }

    /// 追加一个 echo 字符。`display` 是渲染时使用的字符串（控制字符可用
    /// caret notation 如 `^H` `^M` `^I`），`byte_len` 是该字符在 input 字节流
    /// 里占用的字节数。
    pub fn push_echo(&mut self, display: impl Into<String>, byte_len: u8) {
        if byte_len == 0 {
            return;
        }
        self.console_echo.push(EchoChar {
            display: display.into(),
            bytes: byte_len,
        });
    }

    /// 按 `vm.console.input` 队列当前长度从 echo 头部弹掉已被程序消费的字符。
    /// 渲染前调用一次。
    pub fn sync_echo(&mut self) {
        let pending = self
            .vm
            .as_ref()
            .map(|vm| vm.console.input_len())
            .unwrap_or(0);
        let echo_bytes: usize = self.console_echo.iter().map(|e| e.bytes as usize).sum();
        if echo_bytes <= pending {
            return;
        }
        let mut to_drop = echo_bytes - pending;
        let mut idx = 0usize;
        while to_drop > 0 && idx < self.console_echo.len() {
            let b = self.console_echo[idx].bytes as usize;
            if b > to_drop {
                // 不可能：echo 字符要么整个被消费要么没有，bytes 是原子单位。
                // 防御性地把它也算消费掉。
                idx += 1;
                break;
            }
            to_drop -= b;
            idx += 1;
        }
        self.console_echo.drain(..idx);
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

/// 读寄存器名当前值（不支持 8 位别名——观察 8 位的话观察整个 ax 即可）。
fn read_reg_value(vm: &Vm, name: &str) -> Option<u16> {
    use crate::vm::i8086::cpu::RegRef;
    let r = RegRef::from_name(name)?;
    Some(match r {
        RegRef::R16(r) => vm.cpu.r16(r),
        RegRef::R8(r) => vm.cpu.r8(r) as u16,
    })
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

pub fn run(cli: Cli, program: Program, disk: Option<Vec<u8>>) -> Result<()> {
    install_panic_hook();

    let source_text = fs::read_to_string(&cli.file)?;
    let mut app = App::boot(
        cli.file.clone(),
        source_text,
        program,
        cli.mem_kb,
        cli.max_steps,
        cli.encoding,
        disk,
    );

    let _guard = TerminalGuard::enter()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    while !app.should_quit {
        app.sync_echo();
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
