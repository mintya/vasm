pub mod event;
pub mod keymap;

use std::io;
use std::path::PathBuf;
use std::time::Duration;

use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use crate::cli::Cli;
use crate::error::Result;
use crate::ui;

pub struct App {
    file: PathBuf,
    should_quit: bool,
}

impl App {
    pub fn new(file: PathBuf) -> Self {
        Self {
            file,
            should_quit: false,
        }
    }

    pub fn file_display(&self) -> String {
        self.file
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.file.to_string_lossy().into_owned())
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }
}

struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> Result<Self> {
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen)?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }
}

pub fn run(cli: Cli) -> Result<()> {
    install_panic_hook();

    let _guard = TerminalGuard::enter()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(cli.file);

    while !app.should_quit {
        terminal.draw(|f| ui::render(f, &app))?;
        if let Some(key) = event::poll_key(Duration::from_millis(100))? {
            keymap::handle(key, &mut app);
        }
    }

    Ok(())
}

fn install_panic_hook() {
    let original = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        original(info);
    }));
}
