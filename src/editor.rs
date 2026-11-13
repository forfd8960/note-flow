use std::{
    ffi::OsStr,
    fs,
    io::{Stdout, Write},
    path::PathBuf,
    time::SystemTime,
};

use crossterm::{
    cursor::MoveTo,
    queue,
    terminal::{self, Clear},
};

use crate::{EResult, Key, editor::Mode::Normal};

/// The largest char boundary at or before `idx`.
fn floor_char_boundary(s: &str, idx: usize) -> usize {
    if idx >= s.len() {
        return s.len();
    }
    let mut i = idx;
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

/// Remove the char right before `idx` (a byte offset), returning the byte
/// offset the char started at. Never underflows and never splits a char.
fn remove_char_before(s: &mut String, idx: usize) -> usize {
    let idx = floor_char_boundary(s, idx);
    let start = s[..idx].char_indices().last().map(|(i, _)| i).unwrap_or(0);
    s.remove(start);
    start
}

#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    Normal,  // Navigate, delete, copy, paste, run commands
    Insert,  // insert mode, can editor the content, Type/edit text
    Command, // command mode, Save, quit, search/replace, settings
}

#[derive(Debug)]
pub struct Document {
    pub name: String,
    pub lines: Vec<String>,
}

impl Default for Document {
    fn default() -> Self {
        let dur = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Self {
            name: format!("file_{}", dur),
            lines: vec![],
        }
    }
}

impl Document {
    pub fn sample_doc() -> Self {
        let mut lines = Vec::new();
        for idx in 0..10 {
            lines.push(format!("{}: This is a Line_{}", idx + 1, idx + 1));
        }
        Self {
            name: format!("sample"),
            lines,
        }
    }

    pub fn open(path: &PathBuf) -> EResult<Self> {
        let content = fs::read_to_string(path)?;
        let lines: Vec<String> = content.split("\n").map(String::from).collect();
        let name = path.file_name();
        Ok(Self {
            name: name
                .unwrap_or(OsStr::new("default"))
                .to_string_lossy()
                .to_string(),
            lines,
        })
    }

    pub fn save(&self) -> EResult<()> {
        let p = PathBuf::from(&self.name);
        fs::write(p, self.content())?;
        Ok(())
    }

    pub fn content(&self) -> String {
        self.lines.join("\n").to_string()
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn line(&self, idx: usize) -> String {
        self.lines
            .get(idx)
            .map(|v| v.clone())
            .unwrap_or(format!(""))
    }

    /// insert char: c at pos for line
    pub fn insert_char(&mut self, line: usize, pos: usize, c: char) -> (usize, usize) {
        if line >= self.lines.len() {
            self.lines.resize(line + 1, String::new());
        }

        let insert_pos = floor_char_boundary(&self.lines[line], pos.min(self.lines[line].len()));
        self.lines[line].insert(insert_pos, c);

        (line, insert_pos + c.len_utf8())
    }

    /// insert new line at: post in line
    pub fn insert_new_line(&mut self, line: usize, pos: usize) -> (usize, usize) {
        if line >= self.lines.len() {
            self.lines.push(String::new());
            return (line, 0);
        }

        let ip = floor_char_boundary(&self.lines[line], pos.min(self.lines[line].len()));
        let new_line = self.lines[line].split_off(ip);
        self.lines.insert(line + 1, new_line);
        (line + 1, 0)
    }

    /// delete char before idx: pos
    pub fn delete_char(&mut self, line: usize, pos: usize) -> (usize, usize) {
        if line >= self.lines.len() {
            return (line, 0);
        }

        // the cursor may sit past the end of the line (e.g. it was on a longer
        // line before moving up), so clamp to what the line actually holds.
        let dp = floor_char_boundary(&self.lines[line], pos.min(self.lines[line].len()));

        if dp > 0 {
            let start = remove_char_before(&mut self.lines[line], dp);
            (line, start)
        } else if line > 0 {
            // delete char at the begin of line, put cursor_x at end of prev line
            let l = self.lines.remove(line);
            self.lines[line - 1].push_str(&l);
            (line - 1, self.lines[line - 1].len())
        } else {
            (line, 0)
        }
    }
}

#[derive(Debug)]
pub struct Editor {
    pub doc: Document,
    pub mode: Mode,
    pub row_offset: u16, // the line index shows top line(row 0)
    pub cursor_x: u16,   // col index in the line
    pub cursor_y: u16,   // line index in the document
    pub screen_rows: u16,
    pub screen_cols: u16,
    pub cmd: String,
    pub status: String,
}

impl Editor {
    pub fn new(doc: Document) -> EResult<Self> {
        let (cols, rows) = terminal::size()?;
        Ok(Self {
            doc,
            mode: Normal,
            cursor_x: 0,
            cursor_y: 0,
            row_offset: 0,

            // leave the last row for status
            screen_rows: rows.saturating_sub(1),
            screen_cols: cols,
            cmd: format!(""),
            status: "".to_string(),
        })
    }

    /// byte length of the line at `y`
    fn line_len(&self, y: usize) -> usize {
        self.doc.lines.get(y).map_or(0, String::len)
    }

    /// Keep the cursor inside the line it is on. After a vertical move the
    /// cursor may be past the end of the (possibly shorter) new line.
    fn clamp_cursor_x(&mut self) {
        let len = self.line_len(self.cursor_y as usize);
        let cur = floor_char_boundary(
            self.doc
                .lines
                .get(self.cursor_y as usize)
                .map_or("", String::as_str),
            (self.cursor_x as usize).min(len),
        );
        self.cursor_x = cur as u16;
    }

    fn move_left(&mut self) {
        let y = self.cursor_y as usize;
        let Some(line) = self.doc.lines.get(y) else {
            return;
        };
        let cur = floor_char_boundary(line, (self.cursor_x as usize).min(line.len()));
        if cur == 0 {
            return;
        }
        let prev = line[..cur]
            .char_indices()
            .last()
            .map(|(i, _)| i)
            .unwrap_or(0);
        self.cursor_x = prev as u16;
    }

    fn move_right(&mut self) {
        let y = self.cursor_y as usize;
        let Some(line) = self.doc.lines.get(y) else {
            return;
        };
        let cur = floor_char_boundary(line, (self.cursor_x as usize).min(line.len()));
        let Some(next) = line[cur..].chars().next() else {
            return;
        };
        self.cursor_x = (cur + next.len_utf8()) as u16;
    }

    fn move_by_word(&mut self, doc_len: usize) {
        let y = self.cursor_y as usize;
        let Some(line) = self.doc.lines.get(y) else {
            return;
        };
        let cur = floor_char_boundary(line, (self.cursor_x as usize).min(line.len()));

        let mut move_len = 0;
        let mut wp = false;
        for ch in line[cur..].chars() {
            if wp && !ch.is_whitespace() {
                break;
            }
            if ch.is_whitespace() {
                wp = true;
            }
            move_len += ch.len_utf8();
        }

        if (cur + move_len) as usize >= line.len() {
            self.cursor_x = 0;
            self.move_down(doc_len);
            return;
        }

        self.cursor_x = (cur + move_len) as u16;
    }

    fn move_backward_by_word(&mut self) {
        let y = self.cursor_y as usize;
        let Some(line) = self.doc.lines.get(y) else {
            return;
        };
        let cur = floor_char_boundary(line, (self.cursor_x as usize).min(line.len()));

        if cur == 0 {
            self.move_to_prev_line_end(y);
            return;
        }

        let mut move_len = 0;
        let mut in_word = false;
        let chars: Vec<char> = line[0..cur].chars().rev().collect();
        let mut idx = 0;
        while idx < chars.len() {
            if in_word && chars[idx].is_whitespace() {
                break;
            }

            if !chars[idx].is_whitespace() {
                in_word = true;
            }

            move_len += chars[idx].len_utf8();
            idx += 1;
        }

        if (cur - move_len) as usize == 0 {
            self.cursor_x = 0;
            return;
        }

        if cur < move_len {
            self.move_to_prev_line_end(y);
            return;
        }

        self.cursor_x = (cur - move_len) as u16;
    }

    fn move_to_prev_line_end(&mut self, y: usize) {
        if y == 0 {
            return;
        }

        let Some(prev_line) = self.doc.lines.get(y - 1) else {
            return;
        };

        self.cursor_x = prev_line.len() as u16;
        self.move_up();
    }

    fn move_up(&mut self) {
        if self.cursor_y > 0 {
            self.cursor_y -= 1;
        }
        self.clamp_cursor_x();
    }

    fn move_down(&mut self, doc_len: usize) {
        if (self.cursor_y as usize + 1) < doc_len {
            self.cursor_y = self.cursor_y.saturating_add(1);
        }
        self.clamp_cursor_x();
    }

    fn move_page_up(&mut self) {
        let jump = self.screen_rows;
        self.cursor_y = self.cursor_y.saturating_sub(jump);
        self.clamp_cursor_x();
    }

    fn move_page_down(&mut self, doc_len: usize) {
        let remaining = doc_len.saturating_sub(self.cursor_y as usize + 1);
        self.cursor_y = self
            .cursor_y
            .saturating_add(remaining.min(self.screen_rows as usize) as u16);
        self.clamp_cursor_x();
    }

    fn move_home(&mut self) {
        self.cursor_x = 0;
    }

    fn move_end(&mut self) {
        self.cursor_x = self.line_len(self.cursor_y as usize) as u16;
    }

    fn edit_document(&mut self, edit: impl FnOnce(&mut Document, usize, usize) -> (usize, usize)) {
        let (row, col) = edit(
            &mut self.doc,
            self.cursor_y as usize,
            self.cursor_x as usize,
        );
        self.cursor_y = row as u16;
        self.cursor_x = col as u16;
    }

    fn insert_to_command(&mut self, c: char) {
        self.cmd.push(c);
    }

    fn clear_cmd(&mut self) {
        self.cmd.clear();
    }

    fn del_cmd_char(&mut self) {
        let _ = self.cmd.pop();
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
        if self.screen_rows == 0 {
            self.row_offset = self.cursor_y;
        } else if self.cursor_y as usize >= self.row_offset as usize + self.screen_rows as usize {
            self.row_offset = self.cursor_y - self.screen_rows + 1;
        }
    }

    fn save(&mut self) -> EResult<()> {
        if self.doc.name.is_empty() {
            self.doc.name = "default".to_string();
        }

        self.status = format!("saving {}", self.doc.name);
        self.doc.save()?;
        self.status = format!("content have saved!");
        Ok(())
    }

    fn switch_mode(&mut self, to: Mode) {
        self.mode = to
    }

    pub fn handle_mode(&mut self, key: Key) -> EResult<bool> {
        match self.mode {
            Mode::Normal => self.handle_normal_mode(key),
            Mode::Insert => self.handle_edit_mode(key),
            Mode::Command => self.handle_command_mode(key),
        }
    }

    pub fn handle_normal_mode(&mut self, key: Key) -> EResult<bool> {
        self.handle_document_mode(Mode::Normal, key)
    }

    pub fn handle_edit_mode(&mut self, key: Key) -> EResult<bool> {
        self.handle_document_mode(Mode::Insert, key)
    }

    fn handle_document_mode(&mut self, mode: Mode, key: Key) -> EResult<bool> {
        match (mode, key) {
            (_, Key::Ctrl('s') | Key::Ctrl('S')) => self.save()?,
            (Mode::Normal, Key::Ctrl('q') | Key::Ctrl('Q')) => return Ok(true),
            (Mode::Normal, Key::Char('i')) => {
                self.switch_mode(Mode::Insert);
            }
            (Mode::Normal, Key::Char(':')) => {
                self.clear_cmd();
                self.insert_to_command(':');
                self.switch_mode(Mode::Command);
            }
            (Mode::Normal, Key::Char('w')) => {
                self.move_by_word(self.doc.len());
            }
            (Mode::Normal, Key::Char('b')) => {
                self.move_backward_by_word();
            }
            (Mode::Insert, Key::ESC) => self.switch_mode(Mode::Normal),
            (Mode::Insert, Key::Char(c)) => {
                self.edit_document(|doc, row, col| doc.insert_char(row, col, c));
            }
            (Mode::Insert, Key::Backspace) => self.edit_document(Document::delete_char),
            (Mode::Insert, Key::Enter) => self.edit_document(Document::insert_new_line),
            (_, key) => self.handle_move(key),
        }

        self.scroll();

        Ok(false)
    }

    pub fn handle_command_mode(&mut self, key: Key) -> EResult<bool> {
        match key {
            Key::ESC => self.switch_mode(Mode::Normal),
            Key::Char(c) => self.insert_to_command(c),
            Key::Backspace => self.del_cmd_char(),
            Key::Enter => return self.run_command(),
            _ => {}
        }
        Ok(false)
    }

    fn run_command(&mut self) -> EResult<bool> {
        match self.cmd.as_str().trim() {
            ":q" => return Ok(true),
            ":w" => {
                self.save()?;
                return Ok(false);
            }
            ":wq" => {
                self.save()?;
                return Ok(true);
            }
            _ => {}
        }
        Ok(false)
    }

    fn handle_move(&mut self, key: Key) {
        let doc_len = self.doc.len();
        match key {
            Key::ArrowLeft => self.move_left(),
            Key::ArrowRight => self.move_right(),
            Key::ArrowUp => self.move_up(),
            Key::ArrowDown => {
                self.move_down(doc_len);
            }
            Key::PageUp => self.move_page_up(),
            Key::PageDown => self.move_page_down(doc_len),
            Key::Home => self.move_home(),
            Key::End => self.move_end(),
            _ => {}
        }
    }

    /// re-paint the screen
    pub fn refresh(&mut self, out: &mut Stdout) -> EResult<()> {
        queue!(out, Clear(terminal::ClearType::All))?;

        for row in 0..self.screen_rows {
            let line_idx = self.row_offset as usize + row as usize;
            if line_idx >= self.doc.len() {
                queue!(out, MoveTo(0, row))?;
                write!(out, "~")?;
                continue;
            }

            let line = self.doc.line(line_idx);
            queue!(out, MoveTo(0, row))?;

            if line.len() <= self.screen_cols as usize {
                let visible: String = line
                    .chars()
                    .into_iter()
                    .take(self.screen_cols as usize)
                    .collect();
                write!(out, "{}", visible)?;
            } else {
                let chars = line.chars().collect::<Vec<char>>();
                let line1 = String::from_iter(&chars[0..self.screen_cols as usize]);
                let line2 = String::from_iter(&chars[self.screen_cols as usize..]);
                write!(out, "{}", line1)?;

                // only wrap onto the next row if there is room for it
                if row + 1 < self.screen_rows {
                    queue!(out, MoveTo(0, row + 1))?;
                    write!(out, "{}", line2)?;
                }
            }
        }

        let status_row = self.screen_rows;
        queue!(out, MoveTo(0, status_row))?;
        if self.mode == Mode::Command {
            write!(out, "{}", self.cmd)?;
        } else {
            if !self.status.is_empty() {
                write!(out, "{}", self.status,)?;
            } else {
                write!(
                    out,
                    "{} rows: {} | x: {}, y: {} | row_offset: {}",
                    self.doc.name,
                    self.doc.len(),
                    self.cursor_x,
                    self.cursor_y,
                    self.row_offset
                )?;
            }
        }

        let screen_x = self.screen_cols.saturating_sub(1).min(self.cursor_x);
        let screen_y = self.cursor_y.saturating_sub(self.row_offset);
        queue!(out, MoveTo(screen_x, screen_y))?;
        out.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{Document, Editor, Mode};
    use crate::Key;

    fn editor(mode: Mode, lines: &[&str]) -> Editor {
        Editor {
            doc: Document {
                name: "test".to_string(),
                lines: lines.iter().map(|line| line.to_string()).collect(),
            },
            mode,
            row_offset: 0,
            cursor_x: 0,
            cursor_y: 0,
            screen_rows: 2,
            screen_cols: 80,
            cmd: String::new(),
            status: String::new(),
        }
    }

    #[test]
    fn navigation_and_scrolling_work_in_both_document_modes() {
        for mode in [Mode::Normal, Mode::Insert] {
            let mut editor = editor(mode, &["first", "second", "third", "fourth"]);
            for (key, expected) in [
                (Key::End, (5, 0, 0)),
                (Key::ArrowRight, (5, 0, 0)),
                (Key::Home, (0, 0, 0)),
                (Key::ArrowLeft, (0, 0, 0)),
                (Key::ArrowRight, (1, 0, 0)),
                (Key::ArrowDown, (1, 1, 0)),
                (Key::PageDown, (1, 3, 2)),
                (Key::ArrowDown, (1, 3, 2)),
                (Key::PageUp, (1, 1, 1)),
                (Key::ArrowUp, (1, 0, 0)),
                (Key::ArrowUp, (1, 0, 0)),
            ] {
                assert!(!editor.handle_mode(key).unwrap());
                assert_eq!(
                    (editor.cursor_x, editor.cursor_y, editor.row_offset),
                    expected
                );
            }
        }
    }

    #[test]
    fn page_down_on_an_empty_document_keeps_the_cursor_at_the_start() {
        for mode in [Mode::Normal, Mode::Insert] {
            let mut editor = editor(mode, &[]);
            assert!(!editor.handle_mode(Key::PageDown).unwrap());
            assert_eq!(
                (editor.cursor_x, editor.cursor_y, editor.row_offset),
                (0, 0, 0)
            );
        }
    }

    #[test]
    fn edits_update_the_document_and_cursor() {
        let mut editor = editor(Mode::Normal, &[]);
        for (key, content, cursor) in [
            (Key::Char('i'), "", (0, 0)),
            (Key::Char('a'), "a", (1, 0)),
            (Key::Char('b'), "ab", (2, 0)),
            (Key::Backspace, "a", (1, 0)),
            (Key::Enter, "a\n", (0, 1)),
            (Key::Char('c'), "a\nc", (1, 1)),
            (Key::Backspace, "a\n", (0, 1)),
            (Key::Backspace, "a", (1, 0)),
            (Key::ESC, "a", (1, 0)),
            (Key::Char('x'), "a", (1, 0)),
        ] {
            assert!(!editor.handle_mode(key).unwrap());
            assert_eq!(editor.doc.content(), content);
            assert_eq!((editor.cursor_x, editor.cursor_y), cursor);
        }
        assert_eq!(editor.mode, Mode::Normal);
    }

    #[test]
    fn quit_and_command_keys_remain_mode_specific() {
        let mut editor = editor(Mode::Insert, &[""]);
        assert!(!editor.handle_mode(Key::Ctrl('q')).unwrap());
        assert!(!editor.handle_mode(Key::Char(':')).unwrap());
        assert_eq!(editor.doc.content(), ":");
        editor.handle_mode(Key::ESC).unwrap();
        assert!(editor.handle_mode(Key::Ctrl('Q')).unwrap());

        editor.cmd = "old command".to_string();
        editor.handle_mode(Key::Char(':')).unwrap();
        assert_eq!(editor.mode, Mode::Command);
        assert_eq!(editor.cmd, ":");
        editor.handle_mode(Key::Char('q')).unwrap();
        assert!(editor.handle_mode(Key::Enter).unwrap());
    }

    #[test]
    fn save_errors_propagate_in_both_document_modes() {
        for mode in [Mode::Normal, Mode::Insert] {
            let mut editor = editor(mode, &["text"]);
            // A NUL makes the path invalid without touching the filesystem.
            editor.doc.name = "invalid\0path".to_string();
            for key in [Key::Ctrl('s'), Key::Ctrl('S')] {
                assert!(editor.handle_mode(key).is_err());
                assert_eq!(editor.doc.content(), "text");
                assert!(editor.status.starts_with("saving "));
            }
        }
    }

    #[test]
    fn backspace_on_an_empty_line_after_moving_up_does_not_panic() {
        // Regression: cursor_x stayed at the old column when moving up onto a
        // shorter (empty) line, so delete_char computed `dp = 0` and then
        // underflowed on `dp - 1`.
        let mut editor = editor(Mode::Insert, &["hello world", ""]);
        editor.cursor_y = 0;
        editor.cursor_x = 11;

        editor.handle_mode(Key::ArrowDown).unwrap();
        // the cursor is clamped to the empty line
        assert_eq!(editor.cursor_x, 0);

        assert!(!editor.handle_mode(Key::Backspace).unwrap());
        assert_eq!(editor.doc.content(), "hello world");
        assert_eq!((editor.cursor_x, editor.cursor_y), (11, 0));
    }

    #[test]
    fn backspace_at_the_start_of_a_line_joins_it_with_the_previous_line() {
        let mut editor = editor(Mode::Insert, &["first", "second"]);
        editor.cursor_y = 1;
        editor.cursor_x = 0;

        editor.handle_mode(Key::Backspace).unwrap();
        assert_eq!(editor.doc.content(), "firstsecond");
        assert_eq!((editor.cursor_x, editor.cursor_y), (11, 0));
    }

    #[test]
    fn backspace_past_the_end_of_a_line_deletes_the_last_char() {
        // A stale cursor_x is clamped to the end of the line, so backspace
        // removes the line's last character instead of panicking.
        let mut editor = editor(Mode::Insert, &["first", "second"]);
        editor.cursor_y = 1;
        editor.cursor_x = 99;

        assert!(!editor.handle_mode(Key::Backspace).unwrap());
        assert_eq!(editor.doc.content(), "first\nsecon");
        assert_eq!((editor.cursor_x, editor.cursor_y), (5, 1));
    }

    #[test]
    fn backspace_on_an_empty_line_with_a_stale_column_does_not_panic() {
        // The reported panic: an empty line with cursor_x > 0 made
        // delete_char compute dp = 0 and then underflow on `dp - 1`.
        let mut editor = editor(Mode::Insert, &["hello world", ""]);
        editor.cursor_y = 1;
        editor.cursor_x = 11;

        assert!(!editor.handle_mode(Key::Backspace).unwrap());
        // nothing to delete on the empty line, so it joins the previous one
        assert_eq!(editor.doc.content(), "hello world");
        assert_eq!((editor.cursor_x, editor.cursor_y), (11, 0));
    }

    #[test]
    fn multibyte_text_is_edited_on_char_boundaries() {
        let mut editor = editor(Mode::Insert, &["héllo wörld"]);
        editor.cursor_y = 0;

        // cursor_x is a byte offset: 'h' is 1 byte, 'é' is 2
        for _ in 0..3 {
            editor.handle_mode(Key::ArrowRight).unwrap();
        }
        assert_eq!(editor.cursor_x, 4);

        assert!(!editor.handle_mode(Key::Backspace).unwrap());
        assert_eq!(editor.doc.content(), "hélo wörld");
        assert_eq!(editor.cursor_x, 3);

        // typing inserts at a char boundary, not mid-char
        editor.handle_mode(Key::Char('x')).unwrap();
        assert_eq!(editor.doc.content(), "héxlo wörld");
        assert_eq!(editor.cursor_x, 4);
    }

    #[test]
    fn the_reported_repro_sequence_does_not_panic() {
        // Add lines to a new file with a blank line between the paragraphs,
        // move up onto that blank line, press delete.
        let mut editor = editor(Mode::Insert, &[]);

        for c in "Row Row Your boat,".chars() {
            editor.handle_mode(Key::Char(c)).unwrap();
        }
        editor.handle_mode(Key::Enter).unwrap(); // -> blank line 1
        editor.handle_mode(Key::Enter).unwrap(); // -> blank line 2
        for c in "Gently down the stream".chars() {
            editor.handle_mode(Key::Char(c)).unwrap();
        }

        assert_eq!(editor.cursor_y, 2);
        assert_eq!(editor.cursor_x, 22);

        // go up onto the blank line
        editor.handle_mode(Key::ArrowUp).unwrap();
        assert_eq!(editor.cursor_y, 1);
        // the column is clamped to the blank line, so delete joins it with
        // the previous line instead of underflowing on `dp - 1`
        assert_eq!(editor.cursor_x, 0);

        // and press delete (the macOS delete key reports as Backspace)
        editor.handle_mode(Key::Backspace).unwrap();

        assert_eq!(editor.doc.lines[0], "Row Row Your boat,");
        assert_eq!(editor.doc.lines[1], "Gently down the stream");
        assert_eq!(editor.cursor_y, 0);
    }

    #[test]
    fn test_split_line() {
        let line = "Awesome text, a long long long long sentence".to_string();
        let part1 = &line[0..10].to_string();
        let part2 = &line[10..].to_string();

        assert_eq!(*part1, "Awesome te".to_string());
        println!("{}", part2);
    }
}
