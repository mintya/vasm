use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use crate::app::{App, InputMode};

pub fn render(area: Rect, buf: &mut Buffer, app: &App) {
    let line = match app.mode() {
        InputMode::Control => Line::from(vec![
            key_chip("Tab"),
            Span::raw(" 切焦点  "),
            key_chip("F1/F2/F3"),
            Span::raw(" 跳 Source/Console/Registers  "),
            key_chip("↑↓PgUp/PgDn"),
            Span::raw(" 滚动  "),
            key_chip("e"),
            Span::raw(" 编辑  "),
            key_chip("q"),
            Span::raw(" 退出"),
        ]),
        InputMode::Input => Line::from(vec![
            Span::styled(
                "[INPUT MODE] ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            key_chip("Esc"),
            Span::raw(" 退出输入  "),
            key_chip("Ctrl-C"),
            Span::raw(" 强制退出"),
        ]),
    };
    Paragraph::new(line).render(area, buf);
}

fn key_chip(label: &str) -> Span<'static> {
    Span::styled(
        format!("[{label}]"),
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )
}
