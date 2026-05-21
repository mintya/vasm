use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

use crate::app::{App, FocusPane};
use crate::vm::i8086::memory::Memory;

const BYTES_PER_ROW: u16 = 16;

pub fn render(area: Rect, buf: &mut Buffer, app: &App) {
    let (seg, off) = app.memory_origin();
    let theme = app.theme();
    let mut block = Block::default()
        .title(format!("Memory  {seg:04X}:{off:04X}"))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border));
    if app.focus() == FocusPane::Memory {
        block = block.border_style(Style::default().fg(theme.border_focused));
    }

    let inner_rows = area.height.saturating_sub(2);
    let lines = match app.vm() {
        Some(vm) => dump_lines(seg, off, &vm.mem, inner_rows),
        None => vec![Line::from("(no vm)")],
    };

    Paragraph::new(lines).block(block).render(area, buf);
}

fn dump_lines(seg: u16, base_off: u16, mem: &Memory, rows: u16) -> Vec<Line<'static>> {
    let mut out = Vec::with_capacity(rows as usize);
    for row in 0..rows {
        let row_off = base_off as u32 + (row as u32) * BYTES_PER_ROW as u32;
        if row_off > u16::MAX as u32 {
            break;
        }
        let row_off_u16 = row_off as u16;
        let mut hex = String::with_capacity(BYTES_PER_ROW as usize * 3 + 2);
        let mut ascii = String::with_capacity(BYTES_PER_ROW as usize);
        for col in 0..BYTES_PER_ROW {
            let off = row_off + col as u32;
            if off > u16::MAX as u32 {
                hex.push_str("   ");
                ascii.push(' ');
                continue;
            }
            let phys = Memory::phys(seg, off as u16);
            let byte = mem.read_u8(phys).ok();
            match byte {
                Some(b) => {
                    hex.push_str(&format!("{b:02X} "));
                    ascii.push(printable(b));
                }
                None => {
                    hex.push_str("-- ");
                    ascii.push(' ');
                }
            }
            if col == 7 {
                hex.push(' ');
            }
        }
        out.push(Line::from(format!("{row_off_u16:04X}  {hex} {ascii}")));
    }
    out
}

fn printable(b: u8) -> char {
    if (0x20..0x7F).contains(&b) {
        b as char
    } else {
        '.'
    }
}
