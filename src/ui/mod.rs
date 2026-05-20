pub mod panes;

use ratatui::Frame;
use ratatui::widgets::{Block, Borders};

use crate::app::App;

pub fn render(frame: &mut Frame, app: &App) {
    let title = format!(" VisualASM  {} ", app.file_display());
    let block = Block::default().title(title).borders(Borders::ALL);
    frame.render_widget(block, frame.area());
}
