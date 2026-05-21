use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

use crate::app::{App, FocusPane};

pub fn render(area: Rect, buf: &mut Buffer, app: &App) {
    let theme = app.theme();
    let frames = app.call_stack();
    let title = format!(" Call Stack [F4] ({}) ", frames.len());
    let mut block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border));
    if app.focus() == FocusPane::CallStack {
        block = block.border_style(Style::default().fg(theme.border_focused));
    }

    let lines: Vec<Line<'static>> = if frames.is_empty() {
        vec![Line::from(Span::styled(
            "(empty — call 后生成栈帧)",
            Style::default().fg(theme.muted),
        ))]
    } else {
        frames
            .iter()
            .enumerate()
            .rev() // 最新在底部
            .map(|(i, f)| {
                let from = f.from_line.map(|l| format!(" L{l}")).unwrap_or_default();
                Line::from(format!(
                    "#{i} return → {:04X}:{:04X}{from}",
                    f.return_cs, f.return_ip
                ))
            })
            .collect()
    };

    Paragraph::new(lines)
        .block(block)
        .scroll((app.call_stack_scroll(), 0))
        .render(area, buf);
}
