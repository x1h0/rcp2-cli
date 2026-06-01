use crossterm::event::{self, Event, KeyEvent};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use std::time::Duration;

pub struct RawModeGuard;

impl RawModeGuard {
    /// # Errors
    /// Returns an error if raw mode cannot be enabled.
    pub fn enter() -> std::io::Result<Self> {
        enable_raw_mode()?;
        Ok(RawModeGuard)
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
    }
}

/// # Errors
/// Returns an error if reading from the terminal fails.
pub fn poll_key(timeout: Duration) -> std::io::Result<Option<KeyEvent>> {
    if event::poll(timeout)?
        && let Event::Key(key) = event::read()? {
            return Ok(Some(key));
        }
    Ok(None)
}
