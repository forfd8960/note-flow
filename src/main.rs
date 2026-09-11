use std::{
    env,
    io::{self},
    path::PathBuf,
};

use crossterm::{
    cursor::{Hide, Show},
    event::{self, Event},
    execute,
    terminal::{
        Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
        enable_raw_mode,
    },
};
use note_flow::{Document, Editor, Key};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();

    let doc = match args.get(1) {
        Some(p) => Document::open(&PathBuf::from(p))?,
        None => Document::default(),
    };

    enable_raw_mode()?;
    execute!(
        io::stdout(),
        EnterAlternateScreen,
        Show,
        Clear(ClearType::All)
    )?;

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

    execute!(&out, LeaveAlternateScreen, Show)?;
    disable_raw_mode()?;
    Ok(())
}
