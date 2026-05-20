use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

use crate::app::{App, FocusPane};

pub fn render(area: Rect, buf: &mut Buffer, app: &App) {
    let mut block = Block::default().title("Segments").borders(Borders::ALL);
    if app.focus() == FocusPane::Registers {
        block = block.border_style(Style::default().fg(Color::Cyan));
    }

    let lines = match app.vm() {
        Some(vm) => {
            let c = &vm.cpu;
            vec![
                Line::from(format!(
                    "cs={:04X} {}   ds={:04X} {}",
                    c.cs,
                    segment_name(app, c.cs),
                    c.ds,
                    segment_name(app, c.ds),
                )),
                Line::from(format!(
                    "ss={:04X} {}   es={:04X} {}",
                    c.ss,
                    segment_name(app, c.ss),
                    c.es,
                    segment_name(app, c.es),
                )),
            ]
        }
        None => vec![Line::from("(no vm)")],
    };

    Paragraph::new(lines).block(block).render(area, buf);
}

fn segment_name(app: &App, paragraph: u16) -> String {
    if let Some(vm) = app.vm() {
        for seg in vm.program.segments.values() {
            if seg.base_paragraph == paragraph {
                return format!("({})", seg.name);
            }
        }
    }
    String::new()
}
