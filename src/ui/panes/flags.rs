use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

use crate::app::{App, FocusPane};
use crate::theme::Theme;
use crate::vm::i8086::cpu::Flags;

pub fn render(area: Rect, buf: &mut Buffer, app: &App) {
    let theme = app.theme();
    let mut block = Block::default()
        .title("Flags")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border));
    if app.focus() == FocusPane::Registers {
        block = block.border_style(Style::default().fg(theme.border_focused));
    }

    let line = match app.vm() {
        Some(vm) => flags_line(theme, &vm.cpu.flags),
        None => Line::from("(no vm)"),
    };

    Paragraph::new(line).block(block).render(area, buf);
}

fn flags_line(theme: &Theme, f: &Flags) -> Line<'static> {
    let mut spans = Vec::with_capacity(18);
    for (name, on) in [
        ("CF", f.cf),
        ("PF", f.pf),
        ("AF", f.af),
        ("ZF", f.zf),
        ("SF", f.sf),
        ("TF", f.tf),
        ("IF", f.if_),
        ("DF", f.df),
        ("OF", f.of),
    ] {
        let mark = if on { "✓" } else { "·" };
        let style = if on {
            Style::default().fg(theme.flag_set)
        } else {
            Style::default().fg(theme.flag_clear)
        };
        spans.push(Span::raw(name));
        spans.push(Span::styled(mark.to_string(), style));
        spans.push(Span::raw(" "));
    }
    Line::from(spans)
}
