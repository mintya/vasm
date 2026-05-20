use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::app::editor;
use crate::app::{App, FocusPane, InputMode, PromptKind};
use crate::error::Result;

pub fn handle(ev: KeyEvent, app: &mut App) -> Result<()> {
    if ev.kind != KeyEventKind::Press {
        return Ok(());
    }

    // 全局：Ctrl-C 任何模式都退出
    if ev.code == KeyCode::Char('c') && ev.modifiers.contains(KeyModifiers::CONTROL) {
        app.quit();
        return Ok(());
    }

    // Prompt 模式优先级最高
    if app.prompt().is_some() {
        return handle_prompt(ev, app);
    }

    match app.mode() {
        InputMode::Input => handle_input(ev, app),
        InputMode::Control => handle_control(ev, app),
    }
}

fn handle_prompt(ev: KeyEvent, app: &mut App) -> Result<()> {
    match ev.code {
        KeyCode::Esc => app.close_prompt(),
        KeyCode::Enter => app.prompt_submit(),
        KeyCode::Backspace => app.prompt_backspace(),
        KeyCode::Char(c) => app.prompt_push(c),
        _ => {}
    }
    Ok(())
}

fn handle_input(ev: KeyEvent, app: &mut App) -> Result<()> {
    match ev.code {
        KeyCode::Esc => {
            app.set_focus(FocusPane::Source);
        }
        // Console 滚动用 PgUp/PgDn，不入缓冲
        KeyCode::PageUp => app.scroll_console(-5),
        KeyCode::PageDown => app.scroll_console(5),
        KeyCode::Char(c) => {
            let mut buf = [0u8; 4];
            for b in c.encode_utf8(&mut buf).as_bytes() {
                app.push_console_input(*b);
            }
        }
        KeyCode::Enter => app.push_console_input(b'\n'),
        KeyCode::Backspace => app.push_console_input(0x08),
        KeyCode::Tab => app.push_console_input(b'\t'),
        _ => {}
    }
    Ok(())
}

fn handle_control(ev: KeyEvent, app: &mut App) -> Result<()> {
    match (ev.code, ev.modifiers) {
        (KeyCode::Char('q'), KeyModifiers::NONE) => app.quit(),
        (KeyCode::Tab, _) => app.cycle_focus(true),
        (KeyCode::BackTab, _) => app.cycle_focus(false),
        (KeyCode::F(1), _) => app.set_focus(FocusPane::Source),
        (KeyCode::F(2), _) => app.set_focus(FocusPane::Console),
        (KeyCode::F(3), _) => app.set_focus(FocusPane::Registers),
        // 调试控制
        (KeyCode::Char('s'), KeyModifiers::NONE) => app.step_once(),
        (KeyCode::Char('n'), KeyModifiers::NONE) => app.step_over(),
        (KeyCode::Char('c'), KeyModifiers::NONE) => app.run_continue(),
        (KeyCode::Char('b'), KeyModifiers::NONE) => app.toggle_breakpoint_at_cursor(),
        (KeyCode::Char('r'), KeyModifiers::NONE) => app.reset(),
        (KeyCode::Char('g'), KeyModifiers::NONE) => app.open_prompt(PromptKind::Goto),
        // 方向键：Source 焦点 → 移 cursor；Memory 焦点 → 滚屏；其他 no-op
        (KeyCode::Up, _) => arrow(app, -1),
        (KeyCode::Down, _) => arrow(app, 1),
        (KeyCode::PageUp, _) => arrow(app, -10),
        (KeyCode::PageDown, _) => arrow(app, 10),
        (KeyCode::Home, _) if app.focus() == FocusPane::Source => {
            app.cursor_to_line(1);
        }
        (KeyCode::End, _) if app.focus() == FocusPane::Source => {
            let total = app.source_text().lines().count() as u32;
            app.cursor_to_line(total);
        }
        (KeyCode::Char('e'), KeyModifiers::NONE) => {
            let file = app.file().clone();
            editor::launch_editor(&file)?;
            if let Err(err) = app.reload() {
                tracing::warn!("reload failed: {err}");
            }
        }
        _ => {}
    }
    Ok(())
}

fn arrow(app: &mut App, lines: i32) {
    match app.focus() {
        FocusPane::Source => app.move_cursor(lines),
        FocusPane::Memory => app.scroll_memory(lines * 16),
        _ => {}
    }
}
