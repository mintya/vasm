use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

use crate::app::{App, FocusPane};
use crate::theme::Theme;

/// Registers pane 排版：
///
/// ```text
/// General   ax=0006  bx=0006  cx=0000  dx=0000
///           ah=00 al=06   bh=00 bl=06   ch=00 cl=00   dh=00 dl=00
/// Index     si=000A  di=0000  bp=0000  sp=0040
/// Pointer   ip=0023
/// ```
///
/// 标签灰色、寄存器名暗色、值高亮——三档色阶让眼睛先抓"区段"再抓"数值"。
pub fn render(area: Rect, buf: &mut Buffer, app: &App) {
    let theme = app.theme();
    let mut block = Block::default()
        .title(" Registers [F3] ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border));
    if app.focus() == FocusPane::Registers {
        block = block.border_style(Style::default().fg(theme.border_focused));
    }

    let lines = match app.vm() {
        Some(vm) => {
            let c = &vm.cpu;
            vec![
                section_line(
                    theme,
                    "General",
                    &[
                        ("ax", word(c.ax)),
                        ("bx", word(c.bx)),
                        ("cx", word(c.cx)),
                        ("dx", word(c.dx)),
                    ],
                ),
                byte_aliases_line(theme, c.ax, c.bx, c.cx, c.dx),
                section_line(
                    theme,
                    "Index",
                    &[
                        ("si", word(c.si)),
                        ("di", word(c.di)),
                        ("bp", word(c.bp)),
                        ("sp", word(c.sp)),
                    ],
                ),
                section_line(theme, "Pointer", &[("ip", word(c.ip))]),
            ]
        }
        None => vec![Line::from(Span::styled(
            "(no vm)",
            Style::default().fg(theme.muted),
        ))],
    };

    Paragraph::new(lines).block(block).render(area, buf);
}

const LABEL_WIDTH: usize = 9;

fn section_line(theme: &Theme, label: &str, entries: &[(&str, String)]) -> Line<'static> {
    let mut spans = vec![Span::styled(
        format!("{:width$}", label, width = LABEL_WIDTH),
        Style::default()
            .fg(theme.muted)
            .add_modifier(Modifier::BOLD),
    )];
    for (i, (name, value)) in entries.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("  "));
        }
        spans.push(Span::styled(
            format!("{name}="),
            Style::default().fg(theme.register_name),
        ));
        spans.push(Span::styled(
            value.clone(),
            Style::default().fg(theme.register_value),
        ));
    }
    Line::from(spans)
}

/// 8 位别名行：ax/bx/cx/dx 各自一组 `xh=NN xl=NN`，组间用三个空格分隔，
/// 让每组开头与上一行 `ax=XXXX  bx=XXXX  cx=XXXX  dx=XXXX` 的对应字段对齐。
fn byte_aliases_line(theme: &Theme, ax: u16, bx: u16, cx: u16, dx: u16) -> Line<'static> {
    let mut spans = vec![Span::raw(" ".repeat(LABEL_WIDTH))];
    for (i, (h_name, h_val, l_name, l_val)) in [
        ("ah", (ax >> 8) as u8, "al", ax as u8),
        ("bh", (bx >> 8) as u8, "bl", bx as u8),
        ("ch", (cx >> 8) as u8, "cl", cx as u8),
        ("dh", (dx >> 8) as u8, "dl", dx as u8),
    ]
    .iter()
    .enumerate()
    {
        if i > 0 {
            spans.push(Span::raw("   "));
        }
        spans.push(Span::styled(
            format!("{h_name}="),
            Style::default().fg(theme.register_name),
        ));
        spans.push(Span::styled(
            format!("{h_val:02X}"),
            Style::default().fg(theme.register_value),
        ));
        spans.push(Span::raw(" "));
        spans.push(Span::styled(
            format!("{l_name}="),
            Style::default().fg(theme.register_name),
        ));
        spans.push(Span::styled(
            format!("{l_val:02X}"),
            Style::default().fg(theme.register_value),
        ));
    }
    Line::from(spans)
}

fn word(v: u16) -> String {
    format!("{v:04X}")
}
