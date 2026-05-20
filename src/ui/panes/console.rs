use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

use crate::app::{App, FocusPane};

pub fn render(area: Rect, buf: &mut Buffer, app: &App) {
    let mut block = Block::default().title("Console [F2]").borders(Borders::ALL);
    if app.focus() == FocusPane::Console {
        block = block.border_style(Style::default().fg(Color::Yellow));
    }

    let buf_bytes = app.console_input();
    let lines: Vec<Line<'static>> = if buf_bytes.is_empty() {
        vec![Line::from("(no output — int 21h/10h 在 M5 接入)")]
    } else {
        // M3 没有真正消费者，把输入缓冲回显出来便于调试
        let text: String = buf_bytes
            .iter()
            .map(|&b| if b == b'\n' { '\n' } else { b as char })
            .collect();
        text.lines().map(|l| Line::from(l.to_string())).collect()
    };

    Paragraph::new(lines)
        .scroll((app.console_scroll(), 0))
        .block(block)
        .render(area, buf);
}
