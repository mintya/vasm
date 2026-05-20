use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use crate::app::{App, FocusPane, InputMode, RunStatus};

pub fn render(area: Rect, buf: &mut Buffer, app: &App) {
    let waiting_input = app
        .vm()
        .map(|vm| vm.console.waiting_for_input())
        .unwrap_or(false);
    let (status_label, status_style) = match app.status() {
        RunStatus::Paused if waiting_input => (
            "● Paused (waiting input)",
            Style::default().fg(Color::Magenta),
        ),
        RunStatus::Paused => ("● Paused", Style::default().fg(Color::Yellow)),
        RunStatus::Halted => ("● Halted", Style::default().fg(Color::Green)),
        RunStatus::Error(_) => ("● Error", Style::default().fg(Color::Red)),
    };

    let cs_ip = match app.vm() {
        Some(vm) => format!("cs:ip={:04X}:{:04X}", vm.cpu.cs, vm.cpu.ip),
        None => "cs:ip=----:----".to_string(),
    };

    let focus_name = match app.focus() {
        FocusPane::Source => "Source",
        FocusPane::Console => "Console",
        FocusPane::Registers => "Registers",
        FocusPane::Memory => "Memory",
    };

    let mode_name = match app.mode() {
        InputMode::Control => "CTRL",
        InputMode::Input => "INPUT",
    };

    let mut spans = vec![
        Span::styled(
            format!(" vasm  {}  ", app.file_display()),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::styled(status_label, status_style),
    ];
    if let RunStatus::Error(msg) = app.status() {
        spans.push(Span::raw(": "));
        spans.push(Span::styled(
            truncate_msg(msg, 60),
            Style::default().fg(Color::Red),
        ));
    }
    spans.extend([
        Span::raw("  "),
        Span::raw(cs_ip),
        Span::raw("  "),
        Span::raw(format!("#steps={}", app.steps_executed())),
        Span::raw("  "),
        Span::raw(format!(
            "#int={}",
            app.vm().map(|vm| vm.console.interrupts()).unwrap_or(0)
        )),
        Span::raw("  focus="),
        Span::styled(focus_name, Style::default().fg(Color::Cyan)),
        Span::raw("  mode="),
        Span::styled(mode_name, Style::default().fg(Color::Yellow)),
    ]);

    Paragraph::new(Line::from(spans)).render(area, buf);
}

fn truncate_msg(msg: &str, max: usize) -> String {
    if msg.chars().count() <= max {
        msg.to_string()
    } else {
        let head: String = msg.chars().take(max - 1).collect();
        format!("{head}…")
    }
}
