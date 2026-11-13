# Design and implementation of the text editor

This document describes the current working-tree implementation of `note-flow`: a small, single-document, modal terminal editor written in Rust. It focuses on the data model, control flow, editing operations, and terminal rendering. Limitations below describe the code as it exists; proposed improvements are identified separately.

## 1. Architecture

The editor runs synchronously on one thread. Each key event updates application state, then triggers a complete redraw. There is no background processing, UI framework, or separate rendering model.

```text
Terminal key event
       |
       v
main.rs: read event and normalize it into Key
       |
       v
Editor::handle_mode
       |
       +--> Normal handler: navigation, mode changes, save, quit
       +--> Insert handler: navigation, text edits, save
       +--> Command handler: build and execute a command string
       |
       v
Update Document and/or Editor state
       |
       v
Editor::refresh --> queued terminal commands --> stdout.flush()
```

| Source | Responsibility |
| --- | --- |
| [`src/main.rs`](../src/main.rs) | Select the document, initialize the terminal, run the event loop, and restore the terminal on normal exit. |
| [`src/lib.rs`](../src/lib.rs) | Export `Document`, `Editor`, and `EResult`; define the application-level `Key` enum and key conversions. |
| [`src/editor.rs`](../src/editor.rs) | Implement the document buffer, modal state machine, cursor movement, editing, file I/O, scrolling, and rendering. |
| [`src/errors.rs`](../src/errors.rs) | Define typed editor errors using `thiserror`. |

`crossterm` provides terminal events, dimensions, cursor movement, raw mode, and alternate-screen commands. `anyhow` is used at the executable boundary; library operations return `EResult<T>`, an alias for `Result<T, EditorErrors>`.

The main architectural separation is between **document content** and **interactive editor state**. `Document` does not know about terminal dimensions or modes. `Editor` owns a `Document` and coordinates everything needed to interact with it. This is a compact design, but `Editor` currently combines controller, viewport, and renderer responsibilities.

## 2. Key data structures

### Document: a vector of UTF-8 lines

```rust
pub struct Document {
    pub name: String,
    pub lines: Vec<String>,
}
```

Each vector element holds one logical line, excluding its separating `\n`. A line can contain other characters, including a `\r` retained from a CRLF file. The complete document is reconstructed with `lines.join("\n")`.

For example:

```text
File content: "hello\nworld\n"
lines:        ["hello", "world", ""]
```

Using `split("\n")` on load preserves the trailing empty element, so joining the unchanged lines preserves a final newline. A new default document starts with zero lines; opening an empty file produces one empty line. Editing must therefore handle two representations of empty content.

This buffer favors direct access and simple implementation:

| Operation | Cost and behavior |
| --- | --- |
| Access a stored line | Vector indexing is O(1); the public `line()` helper clones the string, adding O(line bytes). |
| Insert a character | `String::insert` may shift the remaining bytes of that line and reallocate. |
| Split a line | `split_off` creates the suffix string; inserting the new vector entry shifts subsequent line entries. |
| Join adjacent lines | Removing a vector entry shifts subsequent entries; appending its text may allocate and copy bytes. |
| Save | Join all lines into a newly allocated string and write the whole file. |

Memory use is proportional to text size plus per-line allocation and vector overhead. There is no rope, gap buffer, piece table, edit history, or incremental save representation. The tradeoff is straightforward code with increasingly expensive edits for very long lines or documents with many lines.

### Editor: content plus session state

```rust
pub struct Editor {
    pub doc: Document,
    pub mode: Mode,
    pub row_offset: u16,
    pub cursor_x: u16,
    pub cursor_y: u16,
    pub screen_rows: u16,
    pub screen_cols: u16,
    pub cmd: String,
    pub status: String,
}
```

| Field | Meaning in the implementation |
| --- | --- |
| `doc` | The one open document. |
| `mode` | Determines how the next key is interpreted. |
| `cursor_y` | Zero-based logical line index in the document. |
| `cursor_x` | Used as a byte offset by document edits, but also as a terminal column during rendering. These units are currently inconsistent for Unicode. |
| `row_offset` | Logical line index displayed at the top of the viewport. |
| `screen_rows` | Terminal height minus one, reserving the last row for status or commands. |
| `screen_cols` | Terminal width captured at construction. |
| `cmd` | Editable command string, initially seeded with `:` when entering Command mode. |
| `status` | Persistent message displayed instead of the default document/cursor status. |

Document methods use `usize` indices, while the editor stores document positions in `u16`. Conversions back to `u16` can truncate large indices; document positions are consequently tied to a terminal-sized integer range.

### Mode and Key: explicit input state

`Mode` has three variants: `Normal`, `Insert`, and `Command`. `Key` abstracts characters, control combinations, navigation keys, and editing keys away from `crossterm` types.

Key normalization uses two steps. `From<KeyEvent>` recognizes Control plus a character; otherwise it returns `Unknown`. The event loop then falls back to `From<KeyCode>` for ordinary characters and special keys. `Key::Delete` is represented but has no editing handler.

The library publicly exports `Editor`, but the `editor` module is crate-private and `Mode` is not re-exported. The state machine is primarily exposed through key handling rather than a conveniently nameable public mode type.

## 3. Startup and terminal lifecycle

`main` opens the first command-line argument as a UTF-8 text file, or constructs a default document named `file_<Unix seconds>`. Opening a nonexistent path returns an error; it does not create a new document for that path.

After loading, startup enables raw mode, enters the alternate screen, shows the cursor, and clears the display. Raw mode allows the application to handle individual keys; the alternate screen provides a temporary full-screen display. `Editor::new` reads terminal dimensions and the application draws its initial frame.

The loop blocks on `event::read()`. Only `Event::Key` events are handled. A handler returns `Ok(true)` to request exit; otherwise the application redraws. Resize and other event variants are ignored, and key event kinds such as press/release are not filtered.

On normal exit, `main` leaves the alternate screen, shows the cursor, and disables raw mode. Cleanup is currently explicit code after the loop. An error propagated with `?` after terminal initialization, or a panic, can bypass it.

## 4. Modal input and commands

| Mode | Input | Result |
| --- | --- | --- |
| Normal | `i` | Enter Insert mode. |
| Normal | `:` | Clear `cmd`, seed it with `:`, and enter Command mode. |
| Normal or Insert | Arrows, Page Up/Down, Home/End | Move the document cursor. |
| Normal or Insert | Ctrl-S | Save. Uppercase `S` is also accepted. |
| Normal | Ctrl-Q | Quit immediately. Uppercase `Q` is also accepted. |
| Insert | Character / Enter / Backspace | Insert text / split a line / delete backward. |
| Insert or Command | Escape | Return to Normal mode. |
| Command | Character / Backspace | Append to / remove the last character of `cmd`. |
| Command | Enter on `:q` | Quit. |
| Command | Enter on `:wq` | Save, then quit. |

`run_command` trims the command string before matching. Other commands, including `:w`, have no effect. They do not clear the buffer, report an error, or leave Command mode. Command backspace can remove the initial colon. Copy, paste, search, replacement, and settings are mentioned in mode comments but are not implemented.

There is no dirty flag or unsaved-change confirmation. Quit actions discard unsaved edits. Once a save succeeds, its status message remains until another operation changes `status`.

## 5. Editing operations and cursor updates

Document mutation methods return `(line, column)`, which the editor copies into its cursor fields. This keeps the resulting cursor position close to the text transformation rather than duplicating all position calculations in input handlers.

**Insert a character.** `insert_char` grows the line vector if needed, clamps the requested position to the line's byte length, inserts with `String::insert`, and returns the insertion position plus `c.len_utf8()`.

**Insert a newline.** `insert_new_line` splits the current string at the requested byte offset and inserts the suffix as the next logical line. The cursor moves to column zero of that next line. For a line index outside the vector, it appends one empty line and returns the requested line index; it does not fill every missing row as character insertion does.

**Backspace within a line.** `delete_char` removes the character starting at `pos - 1` and returns that position. This works for ASCII positions but does not locate the preceding UTF-8 character boundary.

**Backspace at the start of a line.** The current line is removed and appended to the previous line. The implementation returns the previous line's length *after* appending, placing the cursor at the end of the merged text. The usual join-boundary position would instead be the previous line's length before the append.

**Navigation.** Left and right change `cursor_x` by one, and right is bounded by the current string's byte length. Up and down change the logical line without clamping the column to the new line's length. Home sets the column to zero; End uses the line's byte length. Page movement jumps by the viewport height, bounded by the available document lines where the calculations permit.

These methods rely on implicit index assumptions. There is no shared position type enforcing a valid line, a UTF-8 boundary, or a distinction between text offsets and display columns.

## 6. Viewport and rendering

Vertical scrolling keeps the logical cursor within the viewport. After Normal or Insert handling, `scroll()` adjusts `row_offset`:

```text
If cursor_y < row_offset:
    row_offset = cursor_y

If cursor_y >= row_offset + screen_rows:
    row_offset = cursor_y - screen_rows + 1

screen_y = cursor_y - row_offset
```

This mapping assumes one logical line occupies one terminal row. There is no horizontal offset or independent visual-row index.

`refresh()` clears the entire display, walks each viewport row, and fetches the corresponding document line. Rows beyond the document show `~`. The last row displays the command buffer in Command mode, otherwise a status message or document/cursor details. Finally, the renderer moves the cursor and flushes stdout. `queue!` delays flushing terminal commands until the final flush; it does not maintain a retained screen model or diff consecutive frames.

Short lines are collected through a character iterator. For a line whose byte length exceeds the screen width, the renderer collects all characters, splits them at `screen_cols`, and writes the suffix on the next terminal row. This is an incomplete wrapping implementation: the next loop iteration also owns that row, longer suffixes can wrap further, and logical-to-screen cursor mapping does not account for the extra rows.

Rendering clones visible lines and allocates temporary strings; long lines also allocate a character vector. Full redraw is easy to reason about for small files, but output volume and allocations grow with visible text. An edit does not currently produce a damage region or a list of changed rows.

## 7. Persistence and errors

`Document::open` reads the complete file with `fs::read_to_string`. Invalid UTF-8 is rejected through an I/O error. It retains only `path.file_name()` in `name`, losing the original parent directory. `save` writes to `PathBuf::from(name)`, relative to the process's current directory. For example, opening `/tmp/notes/todo.txt` from another directory and saving writes `./todo.txt` there.

Saving allocates the complete joined document and calls `fs::write` directly. It has no temporary-file replacement, backup, external-change detection, or separate save-as path. The editor sets a saving status before I/O and a success status afterward; failure propagates to `main`.

`EditorErrors` wraps `io::Error` and defines `KeyNotSupported`, though the current handlers ignore unsupported keys rather than constructing that error.

## 8. Review findings and design priorities

The current implementation is a useful small baseline for understanding a terminal editor. Its main correctness risks come from coordinate assumptions and implicit state invariants:

| Finding | Concrete consequence | Design direction |
| --- | --- | --- |
| Byte offsets, characters, and terminal columns are mixed. | Moving left after inserting `é` can leave the cursor inside its encoding; insertion, splitting, or deletion can panic. Wide and combining characters also misalign the terminal cursor. | Keep valid text offsets separate from display columns; navigate character or grapheme boundaries and compute terminal cell widths. |
| Long-line layout disagrees with viewport mapping. | Wrapped text can be overwritten by the next logical line or reach the status area. A multibyte line can pass the byte-length check while having too few characters for the slice, causing a panic. | Choose clipping/horizontal scrolling or build an explicit visual-row layout used by both drawing and cursor placement. |
| Cursor validity is not maintained across movement. | Moving down to a shorter line can leave an invalid column; Backspace may delete the wrong character or call `remove` on an empty string. | Centralize position validation and define how vertical movement preserves a preferred column. |
| Empty-document and small-terminal arithmetic is unchecked. | Page Down with zero lines underflows; zero viewport dimensions and large `u16` offsets can also cause invalid calculations. | Define empty-buffer invariants, use `usize` for document positions, and guard viewport arithmetic. |
| Line joining returns the wrong editing position. | Backspace at the start of `world` in `["hello", "world"]` returns column 10 rather than the join boundary at column 5. | Capture the previous line's length before appending. |
| Save discards the original path. | Saving a file opened from another directory can write to an unintended file in the working directory. | Store a `PathBuf` separately from the display name. |
| Terminal restoration depends on normal control flow. | I/O failures or panics can leave raw mode or the alternate screen active. | Own terminal setup and restoration in a lifecycle guard, with appropriate panic cleanup. |
| Terminal dimensions are captured once. | Resizing leaves stale viewport bounds. | Handle resize events and recompute layout and scrolling. |

These changes should precede replacing the buffer with a more complex structure. A line vector can remain appropriate for a small editor once positions, viewport layout, and persistence have clear contracts. If large-file workloads later justify it, buffer operations can be hidden behind an interface before introducing a rope, gap buffer, or piece table.

A practical next separation is: `Document` for text and file identity, a validated position/navigation model for edits, a viewport/layout component for mapping text to screen cells, and a terminal session/renderer for output and cleanup. This would allow buffer and movement behavior to be tested without a real terminal, while keeping `Editor` responsible for mode transitions and orchestration.

## 9. Validation coverage

The repository currently contains one unit test, `test_split_line`, which checks an ASCII string slice. It does not exercise the document mutation methods, modal transitions, scrolling, persistence, or rendering. Passing the existing suite therefore provides little evidence for editor correctness.

High-value future tests would cover UTF-8 boundary movement and deletion, splitting and rejoining lines, cursor movement onto shorter lines, empty-buffer Page Down, saving back to an opened path, and viewport behavior at small sizes. Making rendering accept a generic writer and supplying dimensions independently of `terminal::size()` would also make terminal output easier to verify.
