use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Widget};

use crate::app::App;

/// 居中弹框：仅当 app.prompt() 非空时渲染。
pub fn render(area: Rect, buf: &mut Buffer, app: &App) {
    let Some(prompt) = app.prompt() else {
        return;
    };
    let theme = app.theme();

    let w = (area.width as u32 * 60 / 100)
        .max(40)
        .min(area.width as u32) as u16;
    let h: u16 = 3;
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
        .title(format!(" {} ", prompt.label))
        .borders(Borders::ALL)
        .border_style(
            Style::default()
                .fg(theme.prompt_border)
                .add_modifier(Modifier::BOLD),
        );

    let line = Line::from(vec![
        Span::styled("> ", Style::default().fg(theme.console_cursor)),
        Span::raw(prompt.buffer.clone()),
        Span::styled("_", Style::default().add_modifier(Modifier::SLOW_BLINK)),
    ]);
    Paragraph::new(line).block(block).render(popup, buf);
}
