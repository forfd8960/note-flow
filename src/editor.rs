use std::io::{Stdout, Write};

use crossterm::{
    cursor::MoveTo,
    execute,
    terminal::{self, Clear},
};

use crate::{EResult, Key};

#[derive(Debug)]
pub struct Cursor {
    pub x: u16,
    pub y: u16,
}

#[derive(Debug)]
pub struct Editor {
    pub cursor: Cursor,
    pub screen_rows: u16,
    pub screen_cols: u16,
}

impl Editor {
    pub fn new() -> EResult<Self> {
        let (cols, rows) = terminal::size()?;
        Ok(Self {
            cursor: Cursor { x: 0, y: 0 },
            screen_rows: rows,
            screen_cols: cols,
        })
    }

    fn move_left(&mut self) {
        self.cursor.x -= 1;
    }

    fn move_right(&mut self) {
        self.cursor.x += 1;
    }

    fn move_up(&mut self) {
        self.cursor.y -= 1;
    }

    fn move_down(&mut self) {
        self.cursor.y += 1;
    }

    pub fn handle_key(&mut self, key: Key) -> EResult<bool> {
        match key {
            Key::ArrowLeft => self.move_left(),
            Key::ArrowRight => self.move_right(),
            Key::ArrowUp => self.move_up(),
            Key::ArrowDown => self.move_down(),
            Key::Ctrl('q') | Key::Ctrl('Q') => return Ok(true),
            _ => {}
        }

        Ok(false)
    }

    /// re-paint the screen
    pub fn refresh(&mut self, out: &mut Stdout) -> EResult<()> {
        execute!(out, Clear(terminal::ClearType::All))?;

        //draw some placeholder chars
        execute!(out, MoveTo(0, 0))?;
        for _ in 0..self.screen_cols {
            print!("X");
        }

        // bottom
        execute!(out, MoveTo(0, self.screen_rows - 1))?;
        for _ in 0..self.screen_cols {
            print!("X");
        }
        // left, right board
        for row in 1..self.screen_rows - 1 {
            execute!(out, MoveTo(0, row))?;
            print!("X");

            execute!(out, MoveTo(self.screen_cols - 1, row))?;
            print!("X");
        }

        execute!(out, MoveTo(2, 2))?;
        print!("Postion: ({}, {})", self.cursor.x, self.cursor.y);

        execute!(out, MoveTo(self.cursor.x, self.cursor.y))?;
        out.flush()?;

        Ok(())
    }
}
