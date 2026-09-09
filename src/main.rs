use std::io::{self, Write};

use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, Event},
    execute,
    terminal::{
        Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
        enable_raw_mode,
    },
};
use note_flow::{Document, Editor, Key};

fn main() -> anyhow::Result<()> {
    enable_raw_mode()?;
    execute!(
        io::stdout(),
        EnterAlternateScreen,
        Hide,
        Clear(ClearType::All)
    )?;

    // move to top left with moveto
    let mut out = io::stdout();
    let mut editor = Editor::new(Document::sample_doc())?;
    editor.refresh(&mut out)?;

    loop {
        if let Event::Key(event) = event::read()? {
            let key = match Key::from(event) {
                Key::Unknown => Key::from(event.code),
                other => other,
            };

            let quit = editor.handle_key(key)?;
            if quit {
                break;
            }
            editor.refresh(&mut out)?;
        }
    }

    execute!(&out, LeaveAlternateScreen, Show)?;

    disable_raw_mode()?;
    Ok(())
}
