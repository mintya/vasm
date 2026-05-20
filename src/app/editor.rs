use std::io;
use std::path::Path;
use std::process::Command;

use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};

use crate::error::Result;

/// 调起外部编辑器编辑 `file`，期间暂离 alternate screen + raw mode；编辑器退出后恢复。
pub fn launch_editor(file: &Path) -> Result<()> {
    let editor = pick_editor();

    // 暂离 TUI 模式（确保编辑器看到正常终端）。
    let _ = disable_raw_mode();
    let _ = execute!(io::stdout(), LeaveAlternateScreen);

    let status = Command::new(&editor).arg(file).status();

    // 不管编辑器成败，都恢复 TUI 模式。
    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen)?;

    match status {
        Ok(s) if !s.success() => {
            tracing::warn!("editor {:?} exited with {:?}", editor, s.code());
        }
        Err(e) => {
            tracing::warn!("failed to spawn editor {:?}: {}", editor, e);
        }
        _ => {}
    }
    Ok(())
}

fn pick_editor() -> String {
    if let Ok(e) = std::env::var("EDITOR")
        && !e.is_empty()
    {
        return e;
    }
    if let Ok(e) = std::env::var("VISUAL")
        && !e.is_empty()
    {
        return e;
    }
    if cfg!(windows) {
        "notepad.exe".to_string()
    } else {
        "vi".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pick_editor_respects_env() {
        // unsafe 是 Rust 1.85+ std::env::set_var 的约束
        unsafe {
            std::env::set_var("EDITOR", "myeditor");
        }
        assert_eq!(pick_editor(), "myeditor");
        unsafe {
            std::env::remove_var("EDITOR");
        }
    }
}
