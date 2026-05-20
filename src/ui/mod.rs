pub mod highlight;
pub mod panes;

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Widget};

use crate::app::App;

pub fn render(frame: &mut Frame, app: &App) {
    let full = frame.area();
    let buf = frame.buffer_mut();

    // 最外层圆角边框：标题居中显示文件名
    let title = format!(" VisualASM · {} ", app.file_display());
    let outer = Block::default()
        .title(title)
        .title_alignment(ratatui::layout::Alignment::Center)
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(
            Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        );
    let area = outer.inner(full);
    outer.render(full, buf);

    // 五行：状态栏 / 三栏主区 / 内存 / 解释 / 键位
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // 状态栏
            Constraint::Min(10),   // 主区
            Constraint::Length(8), // 内存
            Constraint::Length(1), // 解释
            Constraint::Length(1), // 键位
        ])
        .split(area);

    panes::status::render(rows[0], buf, app);

    // 主区三栏
    let main = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(38),
            Constraint::Percentage(28),
            Constraint::Percentage(34),
        ])
        .split(rows[1]);

    panes::source::render(main[0], buf, app);

    // 中栏垂直切：Console(上) + CallStack(下)
    let center = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(main[1]);
    panes::console::render(center[0], buf, app);
    panes::call_stack::render(center[1], buf, app);

    // 右栏 4 段：Registers / Segments / Flags / Stack
    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6), // Registers
            Constraint::Length(4), // Segments
            Constraint::Length(3), // Flags
            Constraint::Min(4),    // Stack
        ])
        .split(main[2]);

    panes::registers::render(right[0], buf, app);
    panes::segments::render(right[1], buf, app);
    panes::flags::render(right[2], buf, app);
    panes::stack::render(right[3], buf, app);

    panes::memory::render(rows[2], buf, app);
    panes::explain::render(rows[3], buf, app);
    panes::keymap::render(rows[4], buf, app);
}
