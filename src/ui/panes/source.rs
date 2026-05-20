use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

use crate::app::{App, FocusPane};
use crate::ui::highlight::highlight_line;

pub fn render(area: Rect, buf: &mut Buffer, app: &App) {
    let mut block = Block::default().title("Source [F1]").borders(Borders::ALL);
    if app.focus() == FocusPane::Source {
        block = block.border_style(Style::default().fg(Color::Cyan));
    }

    let inner_rows = area.height.saturating_sub(2) as usize;
    let scroll = app.source_scroll() as usize;
    let hi = app.highlighted_line(); // 1-based
    let mut lines = Vec::with_capacity(inner_rows);
    let src_lines: Vec<&str> = app.source_text().lines().collect();
    let total = src_lines.len();
    let max_line_no = total.max(1);
    let gutter_width = max_line_no.to_string().len();

    for row in 0..inner_rows {
        let line_idx = scroll + row;
        if line_idx >= total {
            break;
        }
        let line_no = (line_idx + 1) as u32;
        let marker = if Some(line_no) == hi { "▶" } else { " " };

        let mut spans: Vec<Span<'static>> = Vec::new();
        spans.push(Span::styled(
            format!("{marker} {line_no:>width$} │ ", width = gutter_width),
            if Some(line_no) == hi {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            },
        ));
        spans.extend(highlight_line(src_lines[line_idx]));
        lines.push(Line::from(spans));
    }

    Paragraph::new(lines).block(block).render(area, buf);
}
