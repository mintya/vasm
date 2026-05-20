use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

use crate::app::{App, FocusPane};

/// Segments pane 排版：每行一个段寄存器，名字定宽 8 字符让多行对齐。
///
/// ```text
/// cs=1001  (code)
/// ds=1000  (data)
/// ss=1002  (stack)
/// es=0000  -
/// ```
pub fn render(area: Rect, buf: &mut Buffer, app: &App) {
    let mut block = Block::default().title(" Segments ").borders(Borders::ALL);
    if app.focus() == FocusPane::Registers {
        block = block.border_style(Style::default().fg(Color::Cyan));
    }

    let lines = match app.vm() {
        Some(vm) => {
            let c = &vm.cpu;
            [("cs", c.cs), ("ds", c.ds), ("ss", c.ss), ("es", c.es)]
                .iter()
                .map(|(name, val)| seg_line(name, *val, app))
                .collect()
        }
        None => vec![Line::from(Span::styled(
            "(no vm)",
            Style::default().fg(Color::DarkGray),
        ))],
    };

    Paragraph::new(lines).block(block).render(area, buf);
}

fn seg_line(name: &str, paragraph: u16, app: &App) -> Line<'static> {
    let label = segment_label(app, paragraph);
    Line::from(vec![
        Span::styled(
            format!("{name}="),
            Style::default()
                .fg(Color::Gray)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{paragraph:04X}"),
            Style::default().fg(Color::White),
        ),
        Span::raw("  "),
        Span::styled(label, Style::default().fg(Color::DarkGray)),
    ])
}

fn segment_label(app: &App, paragraph: u16) -> String {
    if let Some(vm) = app.vm() {
        for seg in vm.program.segments.values() {
            if seg.base_paragraph == paragraph {
                return format!("({})", seg.name);
            }
        }
    }
    "-".to_string()
}
