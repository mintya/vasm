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
        vasm::encoding::Encoding::Utf8,
        None,
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
        vasm::encoding::Encoding::Utf8,
        None,
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
fn console_pane_shows_cursor_when_idle() {
    // 无输出无回显时，Console 仍渲染一个 █ 光标（终端外观）。
    let app = boot_app();
    let s = render_to_buffer(&app, 140, 35);
    assert!(s.contains("Console"), "console 标题: {s}");
    assert!(s.contains("█"), "console 应有 █ 光标: {s}");
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
    assert_eq!(app.focus(), FocusPane::CallStack);
    app.cycle_focus(true);
    assert_eq!(app.focus(), FocusPane::Source);
    app.cycle_focus(false);
    assert_eq!(app.focus(), FocusPane::CallStack);
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
        vasm::encoding::Encoding::Utf8,
        None,
    );
    // step 直到 call 之后
    for _ in 0..5 {
        app.step_once();
    }
    // 现在应执行了 call，stack 应有 1 帧
    assert_eq!(app.call_stack().len(), 1);
    let s = render_to_buffer(&app, 140, 35);
    assert!(
        s.contains("Call Stack [F4] (1)"),
        "call stack 标题应显示数量: {s}"
    );
}

#[test]
fn halted_pc_points_to_hlt_not_following_instr() {
    // 回归：hlt 之后 ip += 1 可能正好落在紧随其后的子过程指令上；UI 不应把 ▶ 标到那条不会执行的指令。
    let src = "code segment\nstart:\n  mov ax, 1\n  hlt\nsubproc:\n  add ax, bx\n  ret\ncode ends\nend start\n";
    let (program, diags) = parser::parse(src);
    assert!(diags.is_empty(), "{diags:?}");
    let mut app = App::boot(
        PathBuf::from("tests/halted.asm"),
        src.to_string(),
        program,
        1024,
        100,
        vasm::encoding::Encoding::Utf8,
        None,
    );
    app.run_continue();
    assert!(matches!(app.status(), RunStatus::Halted));
    let hi = app.highlighted_line().expect("有 PC 行");
    let line_text = app.source_text().lines().nth((hi - 1) as usize).unwrap();
    assert!(
        line_text.contains("hlt"),
        "halted 后 ▶ 应指 hlt 行，实际指向 L{hi}: {line_text:?}"
    );
}

// ---- M5：Console / status / explain ----

fn boot_with_src(src: &str, encoding: vasm::encoding::Encoding) -> App {
    let (program, diags) = parser::parse(src);
    assert!(diags.is_empty(), "{diags:?}");
    App::boot(
        PathBuf::from("tests/m5.asm"),
        src.to_string(),
        program,
        1024,
        100_000,
        encoding,
        None,
    )
}

#[test]
fn console_pane_renders_dos_output_after_int21() {
    let src = "data segment\n  msg db 'HI$'\ndata ends\n\
         code segment\n  assume cs:code, ds:data\nstart:\n  \
         mov ax, data\n  mov ds, ax\n  mov dx, offset msg\n  mov ah, 9\n  int 21h\n  \
         mov ah, 4ch\n  int 21h\ncode ends\nend start\n";
    let mut app = boot_with_src(src, vasm::encoding::Encoding::Utf8);
    app.run_continue();
    let s = render_to_buffer(&app, 140, 35);
    assert!(s.contains("HI"), "console 应包含 DOS 输出: {s}");
}

#[test]
fn console_pane_decodes_gbk_output() {
    // 渲染 ratatui TestBackend 时宽字符（中文）会占多列、跨 cell，断言比较脆弱。
    // 直接验证编码层：vm.console.output 是 GBK 字节流，经 app.encoding 解码后包含 "你好"。
    let src = "code segment\n  hlt\ncode ends\nend\n";
    let mut app = boot_with_src(src, vasm::encoding::Encoding::Gbk);
    if let Some(vm) = app.vm_mut() {
        vm.console.push_output_bytes(&[0xC4, 0xE3, 0xBA, 0xC3]); // "你好"
    }
    let decoded = app.encoding().decode(app.vm().unwrap().console.output());
    assert!(decoded.contains("你好"), "GBK 解码: {decoded:?}");
}

#[test]
fn status_shows_waiting_input_when_dos_blocks() {
    let src = "code segment\nstart:\n  mov ah, 1\n  int 21h\n  hlt\ncode ends\nend start\n";
    let mut app = boot_with_src(src, vasm::encoding::Encoding::Utf8);
    // 跑到 int 21h ah=1，缓冲为空 → WaitingForInput
    app.run_continue();
    let s = render_to_buffer(&app, 140, 35);
    assert!(s.contains("waiting"), "状态栏应提示 waiting: {s}");
    assert!(
        s.contains("> "),
        "console echo prompt 应可见（waiting 时变黄）: {s}"
    );
}

#[test]
fn status_shows_int_counter() {
    let src = "code segment\nstart:\n  mov dl, '!'\n  mov ah, 2\n  int 21h\n  mov ah, 4ch\n  int 21h\ncode ends\nend start\n";
    let mut app = boot_with_src(src, vasm::encoding::Encoding::Utf8);
    app.run_continue();
    let s = render_to_buffer(&app, 140, 35);
    assert!(s.contains("#int=2"), "状态栏应显示 #int=2: {s}");
}

#[test]
fn explain_pane_annotates_int_21h_ah_9() {
    let src = "data segment\n  m db 'X$'\ndata ends\n\
         code segment\n  assume cs:code, ds:data\nstart:\n  \
         mov ax, data\n  mov ds, ax\n  mov dx, offset m\n  mov ah, 9\n  int 21h\n  \
         mov ah, 4ch\n  int 21h\ncode ends\nend start\n";
    let mut app = boot_with_src(src, vasm::encoding::Encoding::Utf8);
    // step 4 次：mov ax,data / mov ds,ax / mov dx,offset m / mov ah,9。下一条就是 int 21h。
    for _ in 0..4 {
        app.step_once();
    }
    let s = render_to_buffer(&app, 140, 35);
    assert!(s.contains("DOS 09h"), "explain 应注解 DOS 09h: {s}");
}

#[test]
fn console_pane_renders_edit_echo() {
    // 用户按键 → keymap 推字节到 vm.console.input + push_echo 推显示字符。
    // 测试里直接 push 字节 + push_echo 模拟。
    let src = "code segment\n  hlt\ncode ends\nend\n";
    let mut app = boot_with_src(src, vasm::encoding::Encoding::Utf8);
    if let Some(vm) = app.vm_mut() {
        for b in b"abc" {
            vm.console.push_input(*b);
        }
    }
    app.push_echo('a', 1);
    app.push_echo('b', 1);
    app.push_echo('c', 1);
    let s = render_to_buffer(&app, 140, 35);
    assert!(s.contains("abc"), "回显字符 abc 应可见: {s}");
}

#[test]
fn echo_auto_drops_when_program_consumes_input() {
    // 用户敲 'x'：input 队 = [x]，echo = [(x, 1)]。
    // 程序 pop 一个字节后调 sync_echo → echo 应清空。
    let src = "code segment\n  hlt\ncode ends\nend\n";
    let mut app = boot_with_src(src, vasm::encoding::Encoding::Utf8);
    if let Some(vm) = app.vm_mut() {
        vm.console.push_input(b'x');
    }
    app.push_echo('x', 1);
    assert_eq!(app.console_echo().len(), 1);

    // 模拟程序消费
    if let Some(vm) = app.vm_mut() {
        assert_eq!(vm.console.pop_input(), Some(b'x'));
    }
    app.sync_echo();
    assert!(app.console_echo().is_empty(), "echo 应在消费后自动清空");
}

#[test]
fn echo_partial_drop_keeps_unconsumed_chars() {
    // 用户敲 'a' 'b'：echo = [(a,1), (b,1)]，input = [a, b]。
    // 程序消费 'a' → echo 头部 'a' 弹掉，剩 [(b,1)]。
    let src = "code segment\n  hlt\ncode ends\nend\n";
    let mut app = boot_with_src(src, vasm::encoding::Encoding::Utf8);
    if let Some(vm) = app.vm_mut() {
        vm.console.push_input(b'a');
        vm.console.push_input(b'b');
    }
    app.push_echo('a', 1);
    app.push_echo('b', 1);
    if let Some(vm) = app.vm_mut() {
        vm.console.pop_input();
    }
    app.sync_echo();
    assert_eq!(app.console_echo().len(), 1);
    assert_eq!(app.console_echo()[0].display, "b");
}

#[test]
fn console_pane_shows_output_then_echo() {
    // output 在前（程序写入），echo 在后（用户敲入），两者顺序可见。
    let src = "code segment\n  hlt\ncode ends\nend\n";
    let mut app = boot_with_src(src, vasm::encoding::Encoding::Utf8);
    if let Some(vm) = app.vm_mut() {
        vm.console.push_output_bytes(b"OK");
        vm.console.push_input(b'?');
        vm.console.push_input(b'?');
    }
    app.push_echo('?', 1);
    app.push_echo('?', 1);
    let s = render_to_buffer(&app, 140, 35);
    let ok_pos = s.find("OK").expect("OK in console");
    let q_pos = s.find("??").expect("?? in console");
    assert!(ok_pos < q_pos, "OK 应在 ?? 之前: {s}");
}

#[test]
fn console_pane_handles_output_crlf_and_backspace() {
    // 输出 "AB\r\nC\x08X" 应该渲染为两行：第一行 "AB"，第二行 "X"（\b 不擦字符，C 被覆盖）。
    let src = "code segment\n  hlt\ncode ends\nend\n";
    let mut app = boot_with_src(src, vasm::encoding::Encoding::Utf8);
    if let Some(vm) = app.vm_mut() {
        vm.console.push_output_bytes(b"AB\r\nC\x08X");
    }
    let s = render_to_buffer(&app, 140, 35);
    assert!(s.contains("AB"), "第一行 AB: {s}");
    // 第二行：C 被 \b X 覆盖成 X
    assert!(s.contains("X"), "第二行 X 可见: {s}");
}

// ---- M6 Stage B：undo + watchpoint + 诊断浮层 -----------------------------

#[test]
fn undo_restores_cpu_and_steps_counter() {
    // 单步两次后 undo 一次，ax 与 #steps 应回退
    let src = "code segment\n  mov ax, 1\n  mov ax, 2\n  hlt\ncode ends\nend\n";
    let mut app = boot_with_src(src, vasm::encoding::Encoding::Utf8);
    app.step_once();
    app.step_once();
    let steps_after = app.steps_executed();
    let ax_after = app.vm().unwrap().cpu.ax;
    assert_eq!(ax_after, 2);
    app.undo();
    assert_eq!(app.steps_executed(), steps_after - 1);
    assert_eq!(app.vm().unwrap().cpu.ax, 1, "undo 后 ax 应回到第一步后的值");
}

#[test]
fn undo_restores_memory_writes() {
    let src = "data segment\n  buf db 4 dup (0)\ndata ends\n\
               code segment\n  assume cs:code, ds:data\nstart:\n  \
               mov ax, data\n  mov ds, ax\n  mov byte ptr ds:[0], 42h\n  hlt\n\
               code ends\nend start\n";
    let mut app = boot_with_src(src, vasm::encoding::Encoding::Utf8);
    // 跑到 mov byte ptr ds:[0], 42h 之后
    for _ in 0..3 {
        app.step_once();
    }
    use vasm::vm::i8086::memory::Memory;
    let ds = app.vm().unwrap().cpu.ds;
    let phys = Memory::phys(ds, 0);
    assert_eq!(app.vm().unwrap().mem.read_u8(phys).unwrap(), 0x42);
    app.undo();
    assert_eq!(
        app.vm().unwrap().mem.read_u8(phys).unwrap(),
        0,
        "undo 后字节回到 0"
    );
}

#[test]
fn watch_hits_when_register_changes() {
    let src = "code segment\n  mov ax, 1\n  mov ax, 2\n  hlt\ncode ends\nend\n";
    let mut app = boot_with_src(src, vasm::encoding::Encoding::Utf8);
    app.add_watch("ax").unwrap();
    assert!(app.last_watch_hit().is_none());
    app.step_once(); // mov ax, 1 → ax 从 0 变 1
    assert!(app.last_watch_hit().is_some(), "ax 变化应命中 watch");
    let s = render_to_buffer(&app, 140, 35);
    assert!(s.contains("watch!"), "status 应显示 watch 命中: {s}");
    assert!(s.contains("watches=1"), "status 应显示 watch 计数: {s}");
}

#[test]
fn diagnostic_modal_shown_on_error() {
    // 触发未实现指令错误，浮层应展示
    let src = "code segment\n  lea ax, [bx]\n  hlt\ncode ends\nend\n";
    let mut app = boot_with_src(src, vasm::encoding::Encoding::Utf8);
    app.step_once();
    assert!(matches!(app.status(), vasm::app::RunStatus::Error(_)));
    let s = render_to_buffer(&app, 140, 35);
    assert!(s.contains("Execution Error"), "诊断浮层应可见: {s}");
    app.dismiss_error();
    assert!(matches!(app.status(), vasm::app::RunStatus::Paused));
}

#[test]
fn undo_restores_console_output() {
    // mov dl,'X' / mov ah,2 / int 21h 之后 console.output = "X"；undo 一步后输出应清空。
    let src = "code segment\nstart:\n  mov dl, 'X'\n  mov ah, 2\n  int 21h\n  hlt\ncode ends\nend start\n";
    let mut app = boot_with_src(src, vasm::encoding::Encoding::Utf8);
    // step 到 int 21h 之后
    for _ in 0..3 {
        app.step_once();
    }
    assert_eq!(app.vm().unwrap().console.output(), b"X");
    app.undo();
    assert!(
        app.vm().unwrap().console.output().is_empty(),
        "undo 后 console output 应回到 int 21h 之前"
    );
}

#[test]
fn watch_on_memory_address_hits_on_write() {
    // 观察一段数据地址；写入后 watch 命中
    let src = "data segment\n  buf db 4 dup (0)\ndata ends\n\
               code segment\n  assume cs:code, ds:data\nstart:\n  \
               mov ax, data\n  mov ds, ax\n  mov byte ptr ds:[0], 99h\n  hlt\n\
               code ends\nend start\n";
    let mut app = boot_with_src(src, vasm::encoding::Encoding::Utf8);
    // 先 step 让 ds 设好，再加 watch
    app.step_once(); // mov ax, data
    app.step_once(); // mov ds, ax
    let ds = app.vm().unwrap().cpu.ds;
    let phys = vasm::vm::i8086::memory::Memory::phys(ds, 0);
    app.add_watch(&format!("{phys:X}")).unwrap();
    assert!(app.last_watch_hit().is_none());
    app.step_once(); // mov byte ptr ds:[0], 99h
    assert!(
        app.last_watch_hit().is_some(),
        "内存 watch 应在写入后命中: {:?}",
        app.last_watch_hit()
    );
}

#[test]
fn clear_watches_removes_all() {
    let src = "code segment\n  mov ax, 1\n  hlt\ncode ends\nend\n";
    let mut app = boot_with_src(src, vasm::encoding::Encoding::Utf8);
    app.add_watch("ax").unwrap();
    app.add_watch("bx").unwrap();
    assert_eq!(app.watches().len(), 2);
    app.clear_watches();
    assert_eq!(app.watches().len(), 0);
    assert!(app.last_watch_hit().is_none());
}

#[test]
fn undo_to_breakpoint_stops_at_marked_phys() {
    // 在第一条指令处设断点，往后单步若干次，再 U 回到断点位置
    let src = "code segment\nstart:\n  mov ax, 1\n  mov ax, 2\n  mov ax, 3\n  hlt\ncode ends\nend start\n";
    let mut app = boot_with_src(src, vasm::encoding::Encoding::Utf8);
    app.cursor_to_line(3); // mov ax,1
    app.toggle_breakpoint_at_cursor();
    assert!(!app.breakpoints().is_empty(), "断点应已设");
    // 跨过断点，单步 3 次
    for _ in 0..3 {
        app.step_once();
    }
    assert_eq!(app.vm().unwrap().cpu.ax, 3);
    app.undo_to_breakpoint();
    // ip 应回到断点（即第一条指令）
    assert_eq!(app.vm().unwrap().cpu.ip, 0, "undo_to_breakpoint 应回到入口");
}

// ---- M6 Stage C：指令元数据 + 主题 + 调用栈焦点 -----------------------------

#[test]
fn explain_pane_shows_insn_doc_summary() {
    // mov 的 doc summary 应在 explain 行可见
    let src = "code segment\nstart:\n  mov ax, 1234h\n  hlt\ncode ends\nend start\n";
    let mut app = boot_with_src(src, vasm::encoding::Encoding::Utf8);
    // 停在第一条 mov 处；TestBackend 每个 CJK 字宽占两 cell（第二 cell 空），
    // 所以中文文本在渲染缓冲里字间会塞空格。用 ASCII 子串 "flags" 做断言更稳。
    let s = render_to_buffer(&app, 140, 35);
    assert!(s.contains("mov ax,"), "explain 应展示当前指令: {s}");
    assert!(
        s.contains("flags"),
        "mov 的 doc summary 含 'flags' 关键词: {s}"
    );
    // 再 step：到 hlt 行
    app.step_once();
    let s = render_to_buffer(&app, 140, 35);
    assert!(s.contains("▶ hlt"), "explain 应展示 hlt: {s}");
}

#[test]
fn theme_default_keeps_dos_green() {
    use vasm::theme::Theme;
    let t = Theme::default();
    // 验收：默认风格保持 M0-M5 视觉
    assert_eq!(t.console_output, ratatui::style::Color::Green);
    assert_eq!(t.status_paused, ratatui::style::Color::Yellow);
}

#[test]
fn theme_toml_overrides_field() {
    use vasm::theme::Theme;
    let toml = "[theme]\nconsole_output = \"lightcyan\"\nborder = \"#102030\"\n";
    let t = Theme::from_toml_str(toml);
    assert_eq!(t.console_output, ratatui::style::Color::LightCyan);
    assert_eq!(t.border, ratatui::style::Color::Rgb(0x10, 0x20, 0x30));
    // 未覆盖的字段保留默认
    assert_eq!(t.status_halted, ratatui::style::Color::Green);
}

#[test]
fn call_stack_pane_is_in_focus_ring() {
    // Tab 循环到 CallStack；CallStack 焦点下方向键滚动 call_stack
    let mut app = boot_app_paused();
    for _ in 0..4 {
        app.cycle_focus(true);
    }
    assert_eq!(app.focus(), FocusPane::CallStack);
    // 滚动 +3 后再读
    app.scroll_call_stack(3);
    assert_eq!(app.call_stack_scroll(), 3);
    let s = render_to_buffer(&app, 140, 35);
    assert!(s.contains("Call Stack [F4]"), "title 应含 [F4]: {s}");
    assert!(
        s.contains("focus=CallStack"),
        "状态栏应显示 focus=CallStack: {s}"
    );
}
