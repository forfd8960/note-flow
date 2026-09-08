use crossterm::event::KeyCode;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EditorErrors {
    #[error("key: {0} not support")]
    KeyNotSupported(KeyCode),
}
