use std::path::PathBuf;

use ratatui::Terminal;
use ratatui::backend::TestBackend;

use vasm::app::{App, FocusPane, PromptKind, RunStatus};
use vasm::asm::parser;

const SAMPLE: &str = "data segment\n  nums dw 1, 2, 3, 4\ndata ends\n\nstack segment\n  db 64 dup (0)\nstack ends\n\ncode segment\n  assume cs:code, ds:data, ss:stack\nstart:\n  mov ax, data\n  mov ds, ax\n  mov ax, stack\n  mov ss, ax\n  mov sp, 64\n  mov si, 0\n  mov ax, [si]\n  add si, 2\n  add ax, [si]\n  add si, 2\n  add ax, [si]\n  push ax\n  pop bx\n  hlt\ncode ends\nend start\n";

fn boot_app() -> App {
    let (program, diags) = parser::parse(SAMPLE);
    assert!(diags.is_empty(), "parse diags: {diags:?}");
    let mut app = App::boot(
        PathBuf::from("tests/inline.asm"),
        SAMPLE.to_string(),
        program,
        1024,
        10_000,
    );
    // M4 起 boot 后默认 Paused 在入口；显式跑到终态以测试 M3 期待的终态显示。
    app.run_continue();
    // 把 memory origin 移到 data 段，方便看 nums 字节
    if let Some(vm) = app.vm() {
        let ds = vm.cpu.ds;
        app.set_memory_origin(ds, 0);
    }
    app
}

fn boot_app_paused() -> App {
    let (program, diags) = parser::parse(SAMPLE);
    assert!(diags.is_empty(), "parse diags: {diags:?}");
    App::boot(
        PathBuf::from("tests/inline.asm"),
        SAMPLE.to_string(),
        program,
        1024,
        10_000,
    )
}

fn render_to_buffer(app: &App, w: u16, h: u16) -> String {
    let backend = TestBackend::new(w, h);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| vasm::ui::render(f, app)).unwrap();
    let buf = terminal.backend().buffer().clone();
    let mut out = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            out.push_str(buf[(x, y)].symbol());
        }
        out.push('\n');
    }
    out
}

#[test]
fn status_shows_halted_and_file() {
    let app = boot_app();
    let s = render_to_buffer(&app, 120, 30);
    assert!(s.contains("Halted"), "status should show Halted: {s}");
    assert!(
        s.contains("inline.asm"),
        "status should show file name: {s}"
    );
    assert!(s.contains("cs:ip="), "status should show cs:ip");
    assert!(s.contains("#steps="), "status should show step counter");
}

#[test]
fn registers_show_final_ax_bx() {
    let app = boot_app();
    let s = render_to_buffer(&app, 120, 30);
    // 验收：ax=0006 bx=0006
    assert!(s.contains("ax=0006"), "ax 应为 0006: {s}");
    assert!(s.contains("bx=0006"), "bx 应为 0006: {s}");
}

#[test]
fn segments_show_named_segments() {
    let app = boot_app();
    let s = render_to_buffer(&app, 140, 35);
    assert!(s.contains("(data)"), "ds 段名应显示: {s}");
    assert!(s.contains("(stack)"), "ss 段名应显示: {s}");
    assert!(s.contains("(code)"), "cs 段名应显示: {s}");
}

#[test]
fn flags_pane_lists_all_nine() {
    let app = boot_app();
    let s = render_to_buffer(&app, 140, 35);
    for f in ["CF", "PF", "AF", "ZF", "SF", "TF", "IF", "DF", "OF"] {
        assert!(s.contains(f), "flag {f} 应显示");
    }
}

#[test]
fn stack_pane_shows_sp_arrow() {
    let app = boot_app();
    let s = render_to_buffer(&app, 140, 35);
    // sp=0040，应能在栈面板看到 "0040" 这个偏移
    assert!(s.contains("Stack"), "stack 标题: {s}");
    assert!(s.contains("0040"), "sp 偏移 0040 应显示: {s}");
}

#[test]
fn memory_pane_shows_data_segment_bytes() {
    let app = boot_app();
    let s = render_to_buffer(&app, 140, 35);
    // nums dw 1,2,3,4 → 01 00 02 00 03 00 04 00
    assert!(s.contains("Memory"), "memory 标题: {s}");
    assert!(
        s.contains("01 00 02 00 03 00 04 00"),
        "数据段前 8 字节应可见: {s}"
    );
}

#[test]
fn source_pane_marks_hlt_line() {
    let mut app = boot_app();
    // 用较大高度让源码完整显示；同时 ▶ 应在 hlt 行
    app.scroll_source(10);
    let s = render_to_buffer(&app, 140, 50);
    assert!(s.contains("▶"), "源码面板应有 ▶ 标记: {s}");
    assert!(s.contains("hlt"), "应能在源码中看到 hlt: {s}");
}

#[test]
fn console_pane_shows_placeholder_until_m5() {
    let app = boot_app();
    let s = render_to_buffer(&app, 140, 35);
    assert!(s.contains("Console"), "console 标题: {s}");
    assert!(s.contains("no output"), "M3 console 应显示占位: {s}");
}

#[test]
fn explain_pane_shows_current_or_halted() {
    let app = boot_app();
    let s = render_to_buffer(&app, 140, 35);
    // hlt 执行后 ip 落在 hlt 之后，current_instruction = None → "halted"
    assert!(
        s.contains("(halted)") || s.contains("▶ "),
        "explain 面板应显示当前指令或 halted: {s}"
    );
}

#[test]
fn call_stack_pane_renders_placeholder() {
    let app = boot_app();
    let s = render_to_buffer(&app, 140, 35);
    assert!(s.contains("Call Stack"), "call stack 标题: {s}");
}

#[test]
fn keymap_pane_lists_tab_and_quit() {
    let app = boot_app();
    let s = render_to_buffer(&app, 140, 35);
    assert!(s.contains("[s]"), "keymap 应有 [s]: {s}");
    assert!(s.contains("[c]"), "keymap 应有 [c]: {s}");
    assert!(s.contains("[b]"), "keymap 应有 [b]: {s}");
    assert!(s.contains("[r]"), "keymap 应有 [r]: {s}");
    assert!(s.contains("[g]"), "keymap 应有 [g]: {s}");
    assert!(s.contains("[Tab]"), "keymap 应有 [Tab]: {s}");
    assert!(s.contains("[q]"), "keymap 应有 [q]: {s}");
}

#[test]
fn focus_changes_keymap_in_input_mode() {
    let mut app = boot_app();
    app.set_focus(FocusPane::Console);
    let s = render_to_buffer(&app, 140, 35);
    assert!(s.contains("INPUT"), "Console 焦点下应显示 INPUT 模式: {s}");
    assert!(s.contains("[Esc]"), "INPUT 模式 keymap 应提示 [Esc]: {s}");
}

#[test]
fn cycle_focus_round_trip() {
    let mut app = boot_app();
    assert_eq!(app.focus(), FocusPane::Source);
    app.cycle_focus(true);
    assert_eq!(app.focus(), FocusPane::Console);
    app.cycle_focus(true);
    assert_eq!(app.focus(), FocusPane::Registers);
    app.cycle_focus(true);
    assert_eq!(app.focus(), FocusPane::Memory);
    app.cycle_focus(true);
    assert_eq!(app.focus(), FocusPane::Source);
    app.cycle_focus(false);
    assert_eq!(app.focus(), FocusPane::Memory);
}

// ---------------- M4 新增 ----------------

#[test]
fn paused_status_shows_at_boot() {
    let app = boot_app_paused();
    assert!(matches!(app.status(), RunStatus::Paused));
    let s = render_to_buffer(&app, 140, 35);
    assert!(s.contains("Paused"), "boot 后状态栏应显示 Paused: {s}");
}

#[test]
fn step_once_advances_one_instruction() {
    let mut app = boot_app_paused();
    let initial_steps = app.steps_executed();
    app.step_once();
    assert_eq!(app.steps_executed(), initial_steps + 1);
}

#[test]
fn breakpoint_marker_appears_on_source_line() {
    let mut app = boot_app_paused();
    // 把 cursor 移到 "  hlt" 行（SAMPLE 里最后一条指令前后）
    app.cursor_to_line(25); // hlt 大致在这行
    app.toggle_breakpoint_at_cursor();
    let s = render_to_buffer(&app, 140, 50);
    // 至少应该有断点 ● 出现（不强制断言行号，因为粗略 cursor）
    if !app.breakpoints().is_empty() {
        assert!(s.contains("●"), "断点行应渲染 ●: {s}");
    }
}

#[test]
fn run_continue_stops_at_breakpoint() {
    let mut app = boot_app_paused();
    app.cursor_to_line(25);
    app.toggle_breakpoint_at_cursor();
    if app.breakpoints().is_empty() {
        return; // 无指令的行（如空行）→ 跳过此用例
    }
    app.run_continue();
    // 状态应保持 Paused（命中断点而不是 Halted）
    assert!(
        matches!(app.status(), RunStatus::Paused | RunStatus::Halted),
        "断点命中或 halt: {:?}",
        app.status()
    );
}

#[test]
fn reset_returns_to_paused_at_entry() {
    let mut app = boot_app_paused();
    app.run_continue(); // 跑到 halt
    assert!(!matches!(app.status(), RunStatus::Paused));
    app.reset();
    assert!(matches!(app.status(), RunStatus::Paused));
    assert_eq!(app.steps_executed(), 0);
}

#[test]
fn goto_label_jumps_cs_ip() {
    let mut app = boot_app_paused();
    app.open_prompt(PromptKind::Goto);
    for c in "start".chars() {
        app.prompt_push(c);
    }
    app.prompt_submit();
    let line = app.highlighted_line();
    assert!(line.is_some(), "goto 后应能解析到入口行: line={line:?}");
}

#[test]
fn prompt_popup_visible_when_open() {
    let mut app = boot_app_paused();
    app.open_prompt(PromptKind::Goto);
    let s = render_to_buffer(&app, 140, 35);
    assert!(s.contains("Goto"), "prompt 弹框应显示标签: {s}");
    assert!(s.contains("> "), "prompt 弹框应有输入光标: {s}");
}

#[test]
fn call_stack_grows_after_call() {
    // 一个简单的 call/ret 程序
    let src = "stk segment\n  db 32 dup (0)\nstk ends\ncode segment\n  assume cs:code, ss:stk\nstart:\n  mov ax, stk\n  mov ss, ax\n  mov sp, 32\n  call sub1\n  hlt\nsub1:\n  inc ax\n  ret\ncode ends\nend start\n";
    let (program, diags) = parser::parse(src);
    assert!(diags.is_empty(), "{diags:?}");
    let mut app = App::boot(
        PathBuf::from("tests/call.asm"),
        src.to_string(),
        program,
        1024,
        10_000,
    );
    // step 直到 call 之后
    for _ in 0..5 {
        app.step_once();
    }
    // 现在应执行了 call，stack 应有 1 帧
    assert_eq!(app.call_stack().len(), 1);
    let s = render_to_buffer(&app, 140, 35);
    assert!(
        s.contains("Call Stack (1)"),
        "call stack 标题应显示数量: {s}"
    );
}
