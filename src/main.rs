use std::io::{Write, stdout};

use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
};

fn main() -> anyhow::Result<()> {
    enable_raw_mode()?;

    print!("we are in raw mode");
    println!();
    print!("Note Flow - A Simple Text Editor");
    println!();

    loop {
        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => break,
                KeyCode::Char(c) => {
                    print!("{}", c);
                }
                KeyCode::Enter => {
                    println!();
                }
                other => print!("{:?}", other),
            }
        }
    }

    disable_raw_mode()?;
    Ok(())
}
