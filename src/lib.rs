use std::io;

use crossterm::cursor::Show;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};

use crate::errors::EditorErrors;

pub(crate) mod editor;
pub(crate) mod errors;

pub use editor::Document;
pub use editor::Editor;

pub type EResult<T> = Result<T, EditorErrors>;

#[derive(Debug, PartialEq)]
pub enum Key {
    Char(char),
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    PageUp,
    PageDown,
    Home,
    End,
    Backspace,
    Delete,
    Enter,
    ESC,
    Ctrl(char),
    Unknown,
}

impl From<KeyCode> for Key {
    fn from(kc: KeyCode) -> Self {
        match kc {
            KeyCode::Char(c) => Key::Char(c),
            KeyCode::Backspace => Key::Backspace,
            KeyCode::Left => Key::ArrowLeft,
            KeyCode::Right => Key::ArrowRight,
            KeyCode::Up => Key::ArrowUp,
            KeyCode::Down => Key::ArrowDown,
            KeyCode::PageUp => Key::PageUp,
            KeyCode::PageDown => Key::PageDown,
            KeyCode::Home => Key::Home,
            KeyCode::End => Key::End,
            KeyCode::Delete => Key::Delete,
            KeyCode::Enter => Key::Enter,
            KeyCode::Esc => Key::ESC,
            _ => Key::Unknown,
        }
    }
}

/// recgonize Ctrl+X
impl From<KeyEvent> for Key {
    fn from(ev: KeyEvent) -> Self {
        if ev.modifiers.contains(KeyModifiers::CONTROL) {
            if let KeyCode::Char(c) = ev.code {
                return Key::Ctrl(c);
            }
        }
        Key::Unknown
    }
}

pub struct TerminalSession;

impl TerminalSession {
    pub fn enter() -> anyhow::Result<Self> {
        enable_raw_mode()?;

        if let Err(e) = execute!(
            io::stdout(),
            EnterAlternateScreen,
            Show,
            Clear(ClearType::All)
        ) {
            let _ = disable_raw_mode();
            return Err(e.into());
        }
        Ok(Self)
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = execute!(io::stdout(), LeaveAlternateScreen, Show);
        let _ = disable_raw_mode();
    }
}
