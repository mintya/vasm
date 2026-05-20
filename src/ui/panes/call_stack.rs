use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

use crate::app::App;

pub fn render(area: Rect, buf: &mut Buffer, _app: &App) {
    let block = Block::default().title("Call Stack").borders(Borders::ALL);
    Paragraph::new("(empty — M4 call/ret 接入后维护)")
        .block(block)
        .render(area, buf);
}
