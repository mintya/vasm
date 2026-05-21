use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

use crate::app::{App, FocusPane};
use crate::ui::highlight::highlight_line;

pub fn render(area: Rect, buf: &mut Buffer, app: &App) {
    let theme = app.theme();
    let mut block = Block::default()
        .title("Source [F1]")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border));
    if app.focus() == FocusPane::Source {
        block = block.border_style(Style::default().fg(theme.border_focused));
    }

    let inner_rows = area.height.saturating_sub(2) as usize;
    let scroll = app.source_scroll() as usize;
    let hi = app.highlighted_line(); // 1-based
    let cursor = app.source_cursor();
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
        let is_cursor = line_no == cursor;
        let is_pc = Some(line_no) == hi;
        let bp = app.line_has_breakpoint(line_no);
        let bp_marker = if bp { "●" } else { " " };
        let pc_marker = if is_pc { "▶" } else { " " };

        let mut spans: Vec<Span<'static>> = Vec::new();
        // gutter 1：bp 标记（红）
        spans.push(Span::styled(
            bp_marker.to_string(),
            if bp {
                Style::default()
                    .fg(theme.source_breakpoint)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            },
        ));
        // gutter 2：pc 标记（黄）+ 行号
        spans.push(Span::styled(
            format!("{pc_marker} {line_no:>width$} │ ", width = gutter_width),
            if is_pc {
                Style::default()
                    .fg(theme.source_pc)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.muted)
            },
        ));
        spans.extend(highlight_line(src_lines[line_idx]));

        // cursor 行：在 Source 焦点时整行加 background 高亮
        let mut line = Line::from(spans);
        if is_cursor && app.focus() == FocusPane::Source {
            line = line.style(Style::default().bg(theme.muted));
        }
        lines.push(line);
    }

    Paragraph::new(lines).block(block).render(area, buf);
}
