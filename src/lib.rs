use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::errors::EditorErrors;

pub(crate) mod editor;
pub(crate) mod errors;

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
