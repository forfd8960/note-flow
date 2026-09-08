use std::io::{Write, stdout};

use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, Event, KeyCode},
    execute,
    terminal::{
        Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
        enable_raw_mode,
    },
};

fn main() -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut out = stdout();
    execute!(stdout(), EnterAlternateScreen, Hide, Clear(ClearType::All))?;

    // move to col, row
    execute!(&out, MoveTo(0, 0))?;
    print!("(0, 0), Top Left");
    execute!(&out, MoveTo(0, 5))?;
    print!("Row 5");
    execute!(&out, MoveTo(39, 0))?;
    print!("Column 39");
    out.flush()?;

    loop {
        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => break,
                KeyCode::Char(c) => {
                    let _ = write_char(&mut out, c)?;
                }
                _ => {}
            }
        }
    }

    execute!(&out, LeaveAlternateScreen, Show)?;
    disable_raw_mode()?;
    Ok(())
}

fn write_char<W: Write>(w: &mut W, c: char) -> anyhow::Result<()> {
    write!(w, "{:?}", c)?;
    w.flush()?;
    Ok(())
}
