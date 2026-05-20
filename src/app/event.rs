use std::time::Duration;

use crossterm::event::{self, Event, KeyEvent};

use crate::error::Result;

pub fn poll_key(timeout: Duration) -> Result<Option<KeyEvent>> {
    if event::poll(timeout)?
        && let Event::Key(k) = event::read()?
    {
        return Ok(Some(k));
    }
    Ok(None)
}
