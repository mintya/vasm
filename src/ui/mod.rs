pub mod highlight;
pub mod panes;

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};

use crate::app::App;

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let buf = frame.buffer_mut();

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
    panes::console::render(main[1], buf, app);

    // 右栏 5 段：Registers / Segments / Flags / Stack / CallStack
    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6), // Registers
            Constraint::Length(4), // Segments
            Constraint::Length(3), // Flags
            Constraint::Min(3),    // Stack
            Constraint::Length(3), // Call Stack
        ])
        .split(main[2]);

    panes::registers::render(right[0], buf, app);
    panes::segments::render(right[1], buf, app);
    panes::flags::render(right[2], buf, app);
    panes::stack::render(right[3], buf, app);
    panes::call_stack::render(right[4], buf, app);

    panes::memory::render(rows[2], buf, app);
    panes::explain::render(rows[3], buf, app);
    panes::keymap::render(rows[4], buf, app);
}
