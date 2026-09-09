use std::io::{Stdout, Write};

use crossterm::{
    cursor::MoveTo,
    execute,
    terminal::{self, Clear},
};

use crate::{EResult, Key};

#[derive(Debug)]
pub struct Document {
    pub lines: Vec<String>,
}

impl Document {
    pub fn sample_doc() -> Self {
        let mut lines = Vec::new();
        for idx in 0..10 {
            lines.push(format!("{}: This is a Line_{}", idx + 1, idx + 1));
        }
        Self { lines }
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn line(&self, idx: usize) -> String {
        let line = self.lines.get(idx);
        match line {
            Some(l) => l.clone(),
            None => format!(""),
        }
    }
}

#[derive(Debug)]
pub struct Editor {
    pub doc: Document,
    pub row_offset: u16, // the line index shows top line(row 0)
    pub cursor_x: u16,   // col index in the line
    pub cursor_y: u16,   // line index in the document
    pub screen_rows: u16,
    pub screen_cols: u16,
}

impl Editor {
    pub fn new(doc: Document) -> EResult<Self> {
        let (cols, rows) = terminal::size()?;
        Ok(Self {
            doc,
            cursor_x: 0,
            cursor_y: 0,
            row_offset: 0,
            screen_rows: rows.saturating_sub(1), // leave the last row for status
            screen_cols: cols,
        })
    }

    fn move_left(&mut self) {
        if self.cursor_x > 0 {
            self.cursor_x -= 1;
        }
    }

    fn move_right(&mut self, line_len: usize) {
        if (self.cursor_x as usize) < line_len {
            self.cursor_x += 1;
        }
    }

    fn move_up(&mut self) {
        if self.cursor_y > 0 {
            self.cursor_y -= 1;
        }
    }

    fn move_down(&mut self, doc_len: usize) {
        if (self.cursor_y as usize + 1) < doc_len {
            self.cursor_y += 1;
        }
    }

    fn move_page_up(&mut self) {
        let jump = self.screen_rows;
        self.cursor_y = self.cursor_y.saturating_sub(jump);
    }

    fn move_page_down(&mut self, doc_len: usize) {
        let jump = self.screen_rows as usize;
        let offset = (doc_len - self.cursor_y as usize) - 1;
        let x = offset.min(jump) as u16;

        self.cursor_y = self.cursor_y.saturating_add(x);
    }

    fn move_home(&mut self) {
        self.cursor_x = 0;
    }

    fn move_end(&mut self, line_count: usize) {
        self.cursor_x = line_count as u16;
    }

    /// scroll adjust the row offset
    /// when scroll up, cursor_y may small than row_offset, so adjust row_offset
    /// to be cursor_y value
    /// when scroll down, cursor_y may greater than row_offset+screen_rows, so adjust row_offset
    /// to be cursor_y - screen_rows + 1
    /// to make sure row_offset show the top line of the doc
    fn scroll(&mut self) {
        // user scroll up, then row_offset also should up
        if self.cursor_y < self.row_offset {
            self.row_offset = self.cursor_y;
        }

        // user scroll down below the screen
        if self.cursor_y >= self.row_offset + self.screen_rows {
            self.row_offset = self.cursor_y - self.screen_rows + 1;
        }
    }

    pub fn handle_key(&mut self, key: Key) -> EResult<bool> {
        let doc_len = self.doc.len();
        let line = self.doc.line(self.cursor_y as usize);
        match key {
            Key::ArrowLeft => self.move_left(),
            Key::ArrowRight => {
                self.move_right(line.len());
            }
            Key::ArrowUp => self.move_up(),
            Key::ArrowDown => {
                self.move_down(doc_len);
            }
            Key::PageUp => self.move_page_up(),
            Key::PageDown => self.move_page_down(doc_len),
            Key::Home => self.move_home(),
            Key::End => self.move_end(line.len()),
            Key::Ctrl('q') | Key::Ctrl('Q') => return Ok(true),
            _ => {}
        }

        self.scroll();

        Ok(false)
    }

    /// re-paint the screen
    pub fn refresh(&mut self, out: &mut Stdout) -> EResult<()> {
        execute!(out, Clear(terminal::ClearType::All))?;

        for row in 0..self.screen_rows {
            let line_idx = self.row_offset + row;
            if line_idx as usize >= self.doc.len() {
                execute!(out, MoveTo(0, row))?;
                print!("~");
                continue;
            }

            let line = self.doc.line(line_idx as usize);
            execute!(out, MoveTo(0, row))?;

            let chars = line.chars().into_iter().take(self.screen_cols as usize);
            print!("{}", String::from_iter(chars));
        }

        let status_row = self.screen_rows;
        execute!(out, MoveTo(0, status_row))?;
        print!(
            "doc rows: {} | x: {}, y: {} | row_offset: {}",
            self.doc.len(),
            self.cursor_x,
            self.cursor_y,
            self.row_offset
        );

        let screen_x = (self.screen_cols - 1).min(self.cursor_x);
        let screen_y = self.cursor_y - self.row_offset;
        execute!(out, MoveTo(screen_x, screen_y))?;
        out.flush()?;
        Ok(())
    }
}
