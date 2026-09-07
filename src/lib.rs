use crate::errors::EditorErrors;

mod errors;

pub type EResult<T> = Result<T, EditorErrors>;

pub struct Editor {}

impl Editor {
    pub fn handle_key(&self) -> EResult<()> {
        Ok(())
    }
}
