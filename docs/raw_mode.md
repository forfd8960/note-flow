In terminal programming, **raw mode** and the **alternate screen** solve two completely different problems:

* **Raw mode** changes **how keyboard input is delivered to your program**.
* **Alternate screen** changes **where terminal output is displayed**.

They are commonly used together by full-screen terminal apps such as `vim`, `less`, `htop`, `tmux`, `lazygit`, and Rust TUI applications built with `crossterm`/`ratatui`.

## 1. What is terminal raw mode?

Normally, your terminal is in what is often called **canonical mode** or **cooked mode**.

Suppose your program does:

```text
read(stdin)
```

and you type:

```text
hello
```

In normal terminal mode, your program typically does **not** receive those characters immediately.

The terminal driver buffers them:

```text
You type:

h
e
l
l
o
Enter
        │
        ▼
┌─────────────────────┐
│ Terminal line buffer│
│ "hello\n"           │
└─────────────────────┘
        │
        ▼
    your program
```

Your program receives something like:

```text
hello\n
```

only after you press Enter.

The terminal also handles several things for you, such as:

```text
Backspace
Ctrl-C
Ctrl-Z
Ctrl-D
echoing typed characters
line editing
```

### Raw mode turns most of that off

In raw mode, keyboard input is delivered much more directly to your program.

For example:

```text
You press: h
              │
              ▼
        your program

You press: e
              │
              ▼
        your program
```

You don't need to press Enter.

This makes things like this possible:

```text
Press ↑ → move cursor up
Press ↓ → move cursor down
Press q → quit
Press j → move selection down
Press Enter → select item
```

That's why terminal UI programs need raw mode.

For example, imagine you're building a file browser:

```text
┌──────────────────────────────┐
│ Files                        │
│                              │
│ > Cargo.toml                 │
│   README.md                  │
│   src/                       │
│   target/                    │
│                              │
│ ↑↓ move   Enter open   q quit│
└──────────────────────────────┘
```

When you press `↓`, the application must receive that key immediately. It cannot wait until you press Enter.

---

## 2. What exactly does raw mode change?

On Unix/Linux/macOS terminals, terminal behavior is controlled by `termios`.

Raw mode generally disables several terminal-driver behaviors.

Conceptually:

```text
Normal mode

keyboard
   │
   ▼
┌──────────────────────┐
│ Terminal driver      │
│                      │
│ line buffering       │
│ echo                 │
│ Ctrl-C processing    │
│ backspace processing │
│ CR/LF translation    │
└──────────────────────┘
   │
   ▼
program
```

Raw mode becomes more like:

```text
keyboard
   │
   ▼
┌───────────────────┐
│ minimal processing│
└───────────────────┘
   │
   ▼
program
```

Several important differences appear.

### Input is no longer line-buffered

Normal:

```text
type: abc

program receives nothing yet

press Enter

program receives:
abc\n
```

Raw:

```text
press a → program receives a
press b → program receives b
press c → program receives c
```

### Input is usually not echoed automatically

Normally:

```text
$ myprogram
hello
^^^^^
terminal displays this automatically
```

In raw mode, typing `hello` may display **nothing** unless your program explicitly draws it.

That is exactly what text editors want.

`vim`, for example, decides itself:

```text
where characters appear
whether characters appear
where the cursor goes
what gets highlighted
```

### Ctrl-C may no longer automatically terminate the process

Normally:

```text
Ctrl-C
```

is interpreted by the terminal driver and turned into:

```text
SIGINT
```

Raw mode can disable this interpretation.

Your application may instead receive the actual key event:

```text
Ctrl + C
```

and decide what to do.

### Backspace becomes your responsibility

In canonical mode:

```text
helol<Backspace><Backspace>lo
```

the terminal driver may perform line editing.

In raw mode, your app usually receives the backspace key itself and must update its own state and redraw the screen.

---

# 3. Arrow keys are interesting in raw mode

Keyboard keys such as letters are straightforward:

```text
a → 0x61
b → 0x62
```

But many special keys are encoded as **escape sequences**.

For example, pressing the Up arrow may produce:

```text
ESC [ A
```

which in bytes is roughly:

```text
0x1B 0x5B 0x41
```

Similarly:

```text
Up       ESC [ A
Down     ESC [ B
Right    ESC [ C
Left     ESC [ D
```

Libraries such as `crossterm` parse these sequences and expose something nicer:

```rust
KeyCode::Up
KeyCode::Down
KeyCode::Left
KeyCode::Right
```

So your application doesn't normally need to manually decode ANSI escape sequences.

---

# 4. What is the terminal main screen?

A terminal emulator normally has a primary display buffer, usually called the:

**main screen**, **primary screen**, or **normal screen**.

Imagine you open a shell:

```text
$ ls
Cargo.toml
src
README.md

$ cargo build
Compiling foo...
Finished dev target...

$ _
```

All of that output lives on your terminal's main screen.

Importantly, it usually contributes to the terminal's **scrollback history**.

You can scroll upward and see:

```text
earlier command
earlier output
older output
...
```

Conceptually:

```text
Main screen buffer

┌──────────────────────────┐
│ older output             │ ↑
│ older output             │ │ scrollback
│ $ ls                     │ │
│ Cargo.toml               │ │
│ src                      │ │
│ $ cargo build            │ │
│ Compiling...             │
│ Finished                 │
│ $                        │ ← current screen
└──────────────────────────┘
```

This is where your shell normally lives.

---

# 5. What is the alternate screen?

Most modern terminal emulators provide a second screen buffer called the:

**alternate screen buffer**.

A program can tell the terminal:

> Switch from the normal screen to a temporary screen.

For example, when you run:

```bash
vim file.txt
```

your terminal may originally look like:

```text
$ ls
foo.txt
bar.txt

$ cargo build
Finished...

$ vim file.txt
```

Then Vim enters the alternate screen:

```text
┌──────────────────────────────────────┐
│ fn main() {                          │
│     println!("hello");               │
│ }                                    │
│                                      │
│                                      │
│ ~                                    │
│ ~                                    │
│                    file.txt  1,1     │
└──────────────────────────────────────┘
```

When you quit Vim:

```text
:q
```

the terminal switches back to the **main screen**:

```text
$ ls
foo.txt
bar.txt

$ cargo build
Finished...

$ vim file.txt
$
```

The Vim UI disappears.

Your previous shell contents are restored.

That is the main purpose of the alternate screen.

---

# 6. Main screen vs alternate screen

The easiest mental model is that your terminal has two canvases:

```text
              Terminal
                  │
        ┌─────────┴─────────┐
        │                   │
        ▼                   ▼
  Main Screen        Alternate Screen
  ┌─────────────┐    ┌─────────────┐
  │ shell       │    │ vim         │
  │ ls          │    │ htop        │
  │ cargo build │    │ less        │
  │ history     │    │ your TUI    │
  └─────────────┘    └─────────────┘
        ▲                   ▲
        │                   │
       normal          temporary
```

A program can switch between them using terminal escape sequences.

The distinction is roughly:

| Main screen                           | Alternate screen                |
| ------------------------------------- | ------------------------------- |
| Normal shell output                   | Temporary full-screen UI        |
| Usually has scrollback                | Usually no normal scrollback    |
| Persists while using shell            | Used temporarily                |
| Commands accumulate here              | App redraws this screen         |
| Restored after alternate screen exits | Usually discarded/restored away |

---

# 7. Why not just draw the TUI on the main screen?

You absolutely can.

Suppose your terminal initially contains:

```text
$ ls
Cargo.toml
src

$ my_app
```

Your program could then draw:

```text
████ MY APPLICATION ████
> item 1
  item 2
  item 3
```

But after quitting, your shell might look like:

```text
$ ls
Cargo.toml
src

$ my_app
████ MY APPLICATION ████
> item 1
  item 2
  item 3
$
```

And if your application continuously redraws, your terminal history might become messy.

An alternate screen prevents this.

```text
before application

MAIN SCREEN
┌────────────────────┐
│ $ ls               │
│ Cargo.toml         │
│ src                │
│ $ my_app           │
└────────────────────┘

        ↓ enter alternate screen

ALTERNATE SCREEN
┌────────────────────┐
│ My App             │
│                    │
│ > item 1           │
│   item 2           │
│   item 3           │
└────────────────────┘

        ↓ exit alternate screen

MAIN SCREEN
┌────────────────────┐
│ $ ls               │
│ Cargo.toml         │
│ src                │
│ $ my_app           │
│ $                  │
└────────────────────┘
```

Your application gets a clean canvas without destroying the user's shell history.

---

# 8. Raw mode and alternate screen are independent

This is an important distinction.

You can enable raw mode **without** entering the alternate screen.

And you can enter the alternate screen **without** enabling raw mode.

They solve independent problems.

```text
Raw mode
    │
    └── controls INPUT behavior

Alternate screen
    │
    └── controls OUTPUT/display buffer
```

A typical TUI enables both:

```text
            Terminal Application

                     │
         ┌───────────┴───────────┐
         ▼                       ▼
     Raw Mode              Alternate Screen
         │                       │
 immediate keyboard        clean UI canvas
 input                     separate from shell
```

For example:

```rust
enable_raw_mode()?;
execute!(stdout(), EnterAlternateScreen)?;
```

Now your app can:

```text
receive ↓ immediately
receive q immediately
receive mouse events
control cursor
redraw screen
```

while keeping the shell's normal screen untouched.

---

# 9. Typical lifecycle of a Rust TUI application

With something like `crossterm` + `ratatui`, the lifecycle often looks like:

```text
Shell
  │
  ▼
enable_raw_mode()
  │
  ▼
EnterAlternateScreen
  │
  ▼
Hide cursor
  │
  ▼
┌──────────────────────┐
│      TUI loop        │
│                      │
│ read key             │
│ update state         │
│ redraw screen        │
│ read key             │
│ update state         │
│ redraw screen        │
└──────────────────────┘
  │
  ▼
Show cursor
  │
  ▼
LeaveAlternateScreen
  │
  ▼
disable_raw_mode()
  │
  ▼
Shell
```

A simplified Rust example:

```rust
use std::io;
use crossterm::{
    execute,
    terminal::{
        enable_raw_mode,
        disable_raw_mode,
        EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};

fn main() -> io::Result<()> {
    enable_raw_mode()?;

    let mut stdout = io::stdout();

    execute!(
        stdout,
        EnterAlternateScreen
    )?;

    // run application
    // ...

    execute!(
        stdout,
        LeaveAlternateScreen
    )?;

    disable_raw_mode()?;

    Ok(())
}
```

The two important operations are conceptually:

```rust
enable_raw_mode();
```

meaning:

> Give my program low-level/immediate keyboard input.

And:

```rust
EnterAlternateScreen
```

meaning:

> Give my program a temporary screen to draw on.

---

# 10. Why cleanup is extremely important

A TUI should always restore the terminal before exiting.

Otherwise, imagine your program crashes while raw mode is still enabled.

You return to the shell and type:

```text
ls
```

but nothing appears because terminal echo is disabled.

Or Ctrl-C may behave strangely.

This is the classic:

> My terminal is broken after my TUI crashed.

Often the terminal itself isn't broken—it was simply left in raw mode.

On Unix systems, running:

```bash
reset
```

or sometimes:

```bash
stty sane
```

can restore normal terminal settings.

Similarly, your program should leave the alternate screen:

```rust
LeaveAlternateScreen
```

otherwise the terminal may remain displaying your application's temporary buffer.

---

# A useful mental model

Think of a terminal application like a video game.

### Raw mode = controller input

Normally the terminal wants you to type a complete line:

```text
north
Enter
```

Raw mode lets the application react instantly:

```text
W → move
A → move
S → move
D → move
q → quit
```

### Alternate screen = game canvas

Instead of printing thousands of lines into your shell history:

```text
player at x=1
player at x=2
player at x=3
player at x=4
...
```

the program gets one screen and repeatedly redraws it:

```text
┌──────────────────────┐
│                      │
│          @           │
│                      │
│ HP: ████████         │
└──────────────────────┘
```

When the game exits, that temporary canvas disappears and your normal shell comes back.

So the shortest definition is:

```text
Raw mode
    = "Let my application handle keyboard input itself."

Alternate screen
    = "Give my application a temporary screen to draw on."
```

And:

```text
Main screen
    = your normal terminal/shell screen + scrollback

Alternate screen
    = temporary full-screen buffer used by applications
```
