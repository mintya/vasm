use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::app::editor;
use crate::app::{App, FocusPane, InputMode};
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

    match app.mode() {
        InputMode::Input => handle_input(ev, app),
        InputMode::Control => handle_control(ev, app),
    }
}

fn handle_input(ev: KeyEvent, app: &mut App) -> Result<()> {
    match ev.code {
        KeyCode::Esc => {
            // 退回控制模式：把焦点切到 Source（任何非 Console 焦点都行）
            app.set_focus(FocusPane::Source);
        }
        KeyCode::Char(c) => {
            // ASCII 字节直接进缓冲；非 ASCII 转换为 UTF-8 字节
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
        (KeyCode::Up, _) => scroll_focus(app, -1),
        (KeyCode::Down, _) => scroll_focus(app, 1),
        (KeyCode::PageUp, _) => scroll_focus(app, -10),
        (KeyCode::PageDown, _) => scroll_focus(app, 10),
        (KeyCode::Char('e'), KeyModifiers::NONE) => {
            let file = app.file().clone();
            editor::launch_editor(&file)?;
            // 编辑器退出后无论成功失败都重载源码
            if let Err(err) = app.reload() {
                tracing::warn!("reload failed: {err}");
            }
        }
        _ => {}
    }
    Ok(())
}

fn scroll_focus(app: &mut App, lines: i32) {
    match app.focus() {
        FocusPane::Source => app.scroll_source(lines),
        FocusPane::Memory => app.scroll_memory(lines * 16),
        // Console 焦点不会走到这里（mode 已切到 Input）
        // Registers 不滚动
        _ => {}
    }
}
