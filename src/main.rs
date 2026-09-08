use std::io::{self, Write};

use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, Event, KeyCode},
    execute,
    terminal::{
        Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
        enable_raw_mode,
    },
};
use note_flow::Key;

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
    execute!(&out, MoveTo(0, 0))?;
    print!("(0,0), Top Left");
    out.flush()?;

    execute!(&out, MoveTo(0, 5))?;
    print!("Row 5");
    out.flush()?;

    execute!(&out, MoveTo(50, 0))?;
    print!("In Column 50");
    out.flush()?;

    execute!(&out, MoveTo(0, 8))?; // show char at row 8

    loop {
        if let Event::Key(event) = event::read()? {
            let key = Key::from(event);
            match key {
                Key::Ctrl('q') | Key::Ctrl('Q') => {
                    break;
                }
                _ => {
                    let kc = Key::from(event.code);
                    execute!(&out, MoveTo(50, 8))?;
                    print!("{:?}", kc);
                    out.flush()?;
                }
            }
        }
    }

    execute!(&out, LeaveAlternateScreen, Show)?;

    disable_raw_mode()?;
    Ok(())
}
