use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use crate::app::{App, InputMode};
use crate::theme::Theme;

pub fn render(area: Rect, buf: &mut Buffer, app: &App) {
    let theme = app.theme();
    if app.prompt().is_some() {
        let line = Line::from(vec![
            Span::styled(
                "[PROMPT] ",
                Style::default()
                    .fg(theme.status_waiting)
                    .add_modifier(Modifier::BOLD),
            ),
            key_chip(theme, "Enter"),
            Span::raw(" 提交  "),
            key_chip(theme, "Esc"),
            Span::raw(" 取消"),
        ]);
        Paragraph::new(line).render(area, buf);
        return;
    }
    let line = match app.mode() {
        InputMode::Control => Line::from(vec![
            key_chip(theme, "s"),
            Span::raw(" 单步 "),
            key_chip(theme, "n"),
            Span::raw(" 步过 "),
            key_chip(theme, "c"),
            Span::raw(" 继续 "),
            key_chip(theme, "b"),
            Span::raw(" 断点 "),
            key_chip(theme, "u"),
            Span::raw(" 撤销 "),
            key_chip(theme, "w"),
            Span::raw(" watch "),
            key_chip(theme, "r"),
            Span::raw(" 复位 "),
            key_chip(theme, "g"),
            Span::raw(" 跳转 "),
            key_chip(theme, "Tab"),
            Span::raw(" 焦点 "),
            key_chip(theme, "e"),
            Span::raw(" 编辑 "),
            key_chip(theme, "q"),
            Span::raw(" 退出"),
        ]),
        InputMode::Input => Line::from(vec![
            Span::styled(
                "[INPUT MODE] ",
                Style::default()
                    .fg(theme.status_paused)
                    .add_modifier(Modifier::BOLD),
            ),
            key_chip(theme, "Esc"),
            Span::raw(" 退出输入  "),
            key_chip(theme, "PgUp/PgDn"),
            Span::raw(" 滚动 Console  "),
            key_chip(theme, "Ctrl-C"),
            Span::raw(" 强制退出"),
        ]),
    };
    Paragraph::new(line).render(area, buf);
}

fn key_chip(theme: &Theme, label: &str) -> Span<'static> {
    Span::styled(
        format!("[{label}]"),
        Style::default()
            .fg(theme.border)
            .add_modifier(Modifier::BOLD),
    )
}
