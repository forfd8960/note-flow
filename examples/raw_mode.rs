use std::io::{self, Write};

use crossterm::terminal::{disable_raw_mode, enable_raw_mode};

fn main() -> anyhow::Result<()> {
    enable_raw_mode()?;

    let mut out = io::stdout();
    write!(out, "Nice to see you\r\n")?;
    write!(out, "Nice to see you again\r\n")?;
    out.flush()?;

    disable_raw_mode()?;
    Ok(())
}
