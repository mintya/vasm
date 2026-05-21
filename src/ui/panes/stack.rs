use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

use crate::app::{App, FocusPane};
use crate::theme::Theme;
use crate::vm::i8086::memory::Memory;

pub fn render(area: Rect, buf: &mut Buffer, app: &App) {
    let theme = app.theme();
    let mut block = Block::default()
        .title("Stack (ss:sp)")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border));
    if app.focus() == FocusPane::Registers {
        block = block.border_style(Style::default().fg(theme.border_focused));
    }

    let lines = match app.vm() {
        Some(vm) => stack_lines(
            theme,
            vm.cpu.ss,
            vm.cpu.sp,
            &vm.mem,
            area.height.saturating_sub(2),
        ),
        None => vec![Line::from("(no vm)")],
    };

    Paragraph::new(lines).block(block).render(area, buf);
}

fn stack_lines(
    theme: &Theme,
    ss: u16,
    sp: u16,
    mem: &Memory,
    max_lines: u16,
) -> Vec<Line<'static>> {
    // 上下各显示一半，sp 行高亮
    let half = (max_lines as i32 / 2).max(1);
    let total = max_lines.max(1) as i32;
    let mut lines = Vec::with_capacity(total as usize);
    for i in 0..total {
        let delta = i - half; // 负数 = 在 sp 之上（较高地址）
        let off = sp as i32 + delta * 2;
        if !(0..=u16::MAX as i32).contains(&off) {
            lines.push(Line::from("       --"));
            continue;
        }
        let off_u16 = off as u16;
        let phys = Memory::phys(ss, off_u16);
        let val_str = match mem.read_u16(phys) {
            Ok(v) => format!("{v:04X}"),
            Err(_) => "----".to_string(),
        };
        let marker = if delta == 0 { "▶" } else { " " };
        let label = format!("{marker} {off_u16:04X}: {val_str}");
        let style = if delta == 0 {
            Style::default()
                .fg(theme.source_pc)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        lines.push(Line::from(Span::styled(label, style)));
    }
    lines
}
