use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::app::App;

pub fn handle(ev: KeyEvent, app: &mut App) {
    if ev.kind != KeyEventKind::Press {
        return;
    }
    match (ev.code, ev.modifiers) {
        (KeyCode::Char('q'), KeyModifiers::NONE) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
            app.quit()
        }
        _ => {}
    }
}
