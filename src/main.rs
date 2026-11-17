use std::{
    env,
    io::{self},
    path::PathBuf,
};

use crossterm::event::{self, Event};

use note_flow::{Document, Editor, Key, TerminalSession};

fn main() -> anyhow::Result<()> {
    // Bind the session to a named variable so it lives until the end of main.
    // Its Drop impl restores the terminal. `let _ = ...` would drop it
    // immediately and restore the terminal before the editor even started.
    let _session = TerminalSession::enter()?;

    let args: Vec<String> = env::args().collect();
    let doc = match args.get(1) {
        Some(p) => Document::open(&PathBuf::from(p))?,
        None => Document::default(),
    };

    // move to top left with moveto
    let mut out = io::stdout();
    let mut editor = Editor::new(doc)?;
    editor.refresh(&mut out)?;

    loop {
        if let Event::Key(event) = event::read()? {
            let key = match Key::from(event) {
                Key::Unknown => Key::from(event.code),
                other => other,
            };

            let quit = editor.handle_mode(key)?;
            if quit {
                break;
            }
            editor.refresh(&mut out)?;
        }
    }

    Ok(())
}
