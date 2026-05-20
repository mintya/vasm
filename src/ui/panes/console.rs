use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

use crate::app::{App, EchoChar, FocusPane};

/// Console pane = 模拟 DOS 风格终端。布局：
///
/// ```text
/// ┌ Console [F2] ──────────────────┐
/// │ output 区（2D grid，绿色）       │
/// │ ...                             │
/// │ ────────────────────────────── │  ← 分隔线
/// │ > ^M abc^H   ▌                │  ← echo 行（青色，超长右截断）
/// └────────────────────────────────┘
/// ```
///
/// - **output 区**按真终端语义渲染 vm.console.output：
///   `\r` 回行首、`\n` 下一行、`\b` 退一格不擦字符、`\t` 制表对齐 8 列、
///   `\x07` (BEL) 忽略、其他 < 0x20 忽略。
/// - **echo 行**显示用户敲过但程序还没消费的字符（控制字符用 caret 形式如 ^H ^M）。
///   程序消费后从 echo 头部自动弹掉（见 `App::sync_echo`）。
/// - waiting 时 echo 行的 prompt 前缀变黄。
pub fn render(area: Rect, buf: &mut Buffer, app: &App) {
    let focused = app.focus() == FocusPane::Console;
    let border_style = Style::default().fg(if focused {
        Color::Yellow
    } else {
        Color::DarkGray
    });
    let block = Block::default()
        .title(" Console [F2] ")
        .borders(Borders::ALL)
        .border_style(border_style)
        .style(Style::default().bg(Color::Black));

    // 先 render 边框，再在 inner area 拆上下区
    let inner = block.inner(area);
    block.render(area, buf);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),    // output
            Constraint::Length(1), // 分隔线
            Constraint::Length(1), // echo prompt
        ])
        .split(inner);

    render_output(chunks[0], buf, app);
    render_separator(chunks[1], buf);
    render_echo_line(chunks[2], buf, app);
}

fn render_output(area: Rect, buf: &mut Buffer, app: &App) {
    let (output_bytes, waiting) = match app.vm() {
        Some(vm) => (vm.console.output().to_vec(), vm.console.waiting_for_input()),
        None => (Vec::new(), false),
    };
    let encoding = app.encoding();

    let mut grid = TerminalGrid::new();
    let decoded = encoding.decode(&output_bytes);
    for ch in decoded.chars() {
        grid.put(ch);
    }

    let output_style = Style::default().fg(Color::Green).bg(Color::Black);
    let cursor_style = Style::default()
        .fg(if waiting { Color::Yellow } else { Color::Green })
        .bg(Color::Black)
        .add_modifier(Modifier::SLOW_BLINK);

    let mut lines = grid.into_lines(output_style);
    lines
        .last_mut()
        .expect("at least one line")
        .spans
        .push(Span::styled("█", cursor_style));

    Paragraph::new(lines)
        .style(Style::default().bg(Color::Black))
        .scroll((app.console_scroll(), 0))
        .render(area, buf);
}

fn render_separator(area: Rect, buf: &mut Buffer) {
    if area.height == 0 {
        return;
    }
    let line = Line::from(Span::styled(
        "─".repeat(area.width as usize),
        Style::default().fg(Color::DarkGray).bg(Color::Black),
    ));
    Paragraph::new(line)
        .style(Style::default().bg(Color::Black))
        .render(area, buf);
}

fn render_echo_line(area: Rect, buf: &mut Buffer, app: &App) {
    let waiting = app
        .vm()
        .map(|vm| vm.console.waiting_for_input())
        .unwrap_or(false);

    let prompt_style = Style::default()
        .fg(if waiting {
            Color::Yellow
        } else {
            Color::DarkGray
        })
        .bg(Color::Black)
        .add_modifier(Modifier::BOLD);
    let echo_style = Style::default()
        .fg(Color::Cyan)
        .bg(Color::Black)
        .add_modifier(Modifier::BOLD);

    // 拼 echo 字符串：每个 EchoChar 一段；超长时右截断保留尾部
    let raw: String = app
        .console_echo()
        .iter()
        .map(|EchoChar { display, .. }| display.as_str())
        .collect();
    let width = area.width.saturating_sub(2) as usize; // 留 "> " 两个字符给 prompt
    let visible = right_trim_to_width(&raw, width);

    let line = Line::from(vec![
        Span::styled("> ", prompt_style),
        Span::styled(visible, echo_style),
    ]);
    Paragraph::new(line)
        .style(Style::default().bg(Color::Black))
        .render(area, buf);
}

/// 把字符串按宽度从右截取（超长时去掉头部），保证最近敲的字符可见。
/// 当前按字符数算，未严格做宽字符（GBK 中文 2 列）计宽——教学场景 echo
/// 通常是 ASCII，足够。
fn right_trim_to_width(s: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= width {
        return s.to_string();
    }
    chars[chars.len() - width..].iter().collect()
}

/// 极简的 DOS Teletype 风格 2D 字符 grid。
struct TerminalGrid {
    rows: Vec<Vec<char>>,
    row: usize,
    col: usize,
}

impl TerminalGrid {
    fn new() -> Self {
        Self {
            rows: vec![Vec::new()],
            row: 0,
            col: 0,
        }
    }

    fn put(&mut self, ch: char) {
        match ch {
            '\r' => self.col = 0,
            '\n' => self.newline(),
            '\u{0008}' => {
                if self.col > 0 {
                    self.col -= 1;
                }
            }
            '\t' => {
                let next = (self.col / 8 + 1) * 8;
                while self.col < next {
                    self.write_at_cursor(' ');
                    self.col += 1;
                }
            }
            '\u{0007}' => {}
            c if (c as u32) < 0x20 => {}
            c => {
                self.write_at_cursor(c);
                self.col += 1;
            }
        }
    }

    fn newline(&mut self) {
        self.row += 1;
        self.col = 0;
        while self.rows.len() <= self.row {
            self.rows.push(Vec::new());
        }
    }

    fn write_at_cursor(&mut self, ch: char) {
        while self.rows.len() <= self.row {
            self.rows.push(Vec::new());
        }
        let line = &mut self.rows[self.row];
        while line.len() <= self.col {
            line.push(' ');
        }
        line[self.col] = ch;
    }

    fn into_lines(self, style: Style) -> Vec<Line<'static>> {
        let mut lines: Vec<Line<'static>> = self
            .rows
            .into_iter()
            .map(|row| {
                if row.is_empty() {
                    Line::default()
                } else {
                    let s: String = row.into_iter().collect();
                    Line::from(Span::styled(s, style))
                }
            })
            .collect();
        if lines.is_empty() {
            lines.push(Line::default());
        }
        lines
    }
}
