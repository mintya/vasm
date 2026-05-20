use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

use crate::app::App;

pub fn render(area: Rect, buf: &mut Buffer, app: &App) {
    let frames = app.call_stack();
    let title = format!("Call Stack ({})", frames.len());
    let block = Block::default().title(title).borders(Borders::ALL);

    let lines: Vec<Line<'static>> = if frames.is_empty() {
        vec![Line::from(Span::styled(
            "(empty — call 后生成栈帧)",
            Style::default().fg(Color::DarkGray),
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

    Paragraph::new(lines).block(block).render(area, buf);
}
