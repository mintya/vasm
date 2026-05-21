use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Widget, Wrap};

use crate::app::{App, RunStatus};

/// 错误浮层：仅当 status = Error 时渲染。按 Enter 关闭（keymap 把 status 切回 Paused）。
pub fn render(area: Rect, buf: &mut Buffer, app: &App) {
    let RunStatus::Error(msg) = app.status() else {
        return;
    };

    let w = (area.width as u32 * 70 / 100)
        .max(50)
        .min(area.width as u32) as u16;
    // 高度 = 边框 2 + 标题前后空 0 + 内容（按 wrap 估算 max 5 行）+ 提示行 1
    let h: u16 = 8;
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    let popup = Rect {
        x,
        y,
        width: w,
        height: h,
    };

    Clear.render(popup, buf);

    let block = Block::default()
        .title(" ✘ Execution Error ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
        .style(Style::default().bg(Color::Black));

    let lines = vec![
        Line::from(Span::styled(
            msg.clone(),
            Style::default().fg(Color::Red).bg(Color::Black),
        )),
        Line::default(),
        Line::from(Span::styled(
            "按 Enter 关闭弹窗（VM 状态保留以便检查）",
            Style::default()
                .fg(Color::DarkGray)
                .bg(Color::Black)
                .add_modifier(Modifier::ITALIC),
        )),
    ];
    Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .block(block)
        .style(Style::default().bg(Color::Black))
        .render(popup, buf);
}
