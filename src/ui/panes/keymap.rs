use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use crate::app::{App, InputMode};

pub fn render(area: Rect, buf: &mut Buffer, app: &App) {
    if app.prompt().is_some() {
        let line = Line::from(vec![
            Span::styled(
                "[PROMPT] ",
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            ),
            key_chip("Enter"),
            Span::raw(" 提交  "),
            key_chip("Esc"),
            Span::raw(" 取消"),
        ]);
        Paragraph::new(line).render(area, buf);
        return;
    }
    let line = match app.mode() {
        InputMode::Control => Line::from(vec![
            key_chip("s"),
            Span::raw(" 单步 "),
            key_chip("n"),
            Span::raw(" 步过 "),
            key_chip("c"),
            Span::raw(" 继续 "),
            key_chip("b"),
            Span::raw(" 断点 "),
            key_chip("r"),
            Span::raw(" 复位 "),
            key_chip("g"),
            Span::raw(" 跳转 "),
            key_chip("Tab"),
            Span::raw(" 焦点 "),
            key_chip("e"),
            Span::raw(" 编辑 "),
            key_chip("q"),
            Span::raw(" 退"),
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
            key_chip("PgUp/PgDn"),
            Span::raw(" 滚动 Console  "),
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
