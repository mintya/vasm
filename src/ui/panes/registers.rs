use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

use crate::app::{App, FocusPane};

pub fn render(area: Rect, buf: &mut Buffer, app: &App) {
    let mut block = Block::default()
        .title("Registers [F3]")
        .borders(Borders::ALL);
    if app.focus() == FocusPane::Registers {
        block = block.border_style(Style::default().fg(Color::Cyan));
    }

    let lines = match app.vm() {
        Some(vm) => {
            let c = &vm.cpu;
            vec![
                Line::from(format!(
                    "ax={:04X}  bx={:04X}  cx={:04X}  dx={:04X}",
                    c.ax, c.bx, c.cx, c.dx
                )),
                Line::from(format!(
                    "si={:04X}  di={:04X}  bp={:04X}  sp={:04X}",
                    c.si, c.di, c.bp, c.sp
                )),
                Line::from(format!(
                    "ip={:04X}  ah={:02X} al={:02X}  bh={:02X} bl={:02X}",
                    c.ip,
                    (c.ax >> 8) as u8,
                    c.ax as u8,
                    (c.bx >> 8) as u8,
                    c.bx as u8,
                )),
                Line::from(format!(
                    "          ch={:02X} cl={:02X}  dh={:02X} dl={:02X}",
                    (c.cx >> 8) as u8,
                    c.cx as u8,
                    (c.dx >> 8) as u8,
                    c.dx as u8,
                )),
            ]
        }
        None => vec![Line::from("(no vm)")],
    };

    Paragraph::new(lines).block(block).render(area, buf);
}
