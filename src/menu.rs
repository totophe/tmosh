//! Minimal raw-terminal selection menu (arrow keys + enter, esc = escape hatch).

use crate::tmux::Session;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor},
    terminal::{self, Clear, ClearType},
};
use std::io::{self, Write};

/// The action chosen by the user.
pub enum Choice {
    /// Attach to an existing session by name.
    Attach(String),
    /// Create a new session (name to be prompted, or empty for default).
    NewSession,
    /// Drop straight to the shell — also the esc/ctrl-c escape hatch.
    Shell,
}

enum Item {
    Session(usize),
    NewSession,
    Shell,
}

/// Renders the menu and blocks until the user makes a choice. Returns
/// `Choice::Shell` for the escape hatch (esc, ctrl-c, or 'q').
pub fn run(sessions: &[Session], version: &str) -> io::Result<Choice> {
    let mut items: Vec<Item> = sessions
        .iter()
        .enumerate()
        .map(|(i, _)| Item::Session(i))
        .collect();
    items.push(Item::NewSession);
    items.push(Item::Shell);

    let mut stderr = io::stderr();
    terminal::enable_raw_mode()?;
    execute!(stderr, cursor::Hide)?;

    let mut selected = 0usize;
    let mut first = true;
    let result = loop {
        draw(&mut stderr, sessions, &items, selected, version, &mut first)?;

        let ev = match event::read() {
            Ok(ev) => ev,
            Err(e) => break Err(e),
        };

        if let Event::Key(KeyEvent {
            code, modifiers, ..
        }) = ev
        {
            match code {
                KeyCode::Up | KeyCode::Char('k') => {
                    selected = selected.checked_sub(1).unwrap_or(items.len() - 1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    selected = (selected + 1) % items.len();
                }
                KeyCode::Esc | KeyCode::Char('q') => break Ok(Choice::Shell),
                KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                    break Ok(Choice::Shell)
                }
                KeyCode::Enter | KeyCode::Char(' ') => {
                    let choice = match items[selected] {
                        Item::Session(i) => Choice::Attach(sessions[i].name.clone()),
                        Item::NewSession => Choice::NewSession,
                        Item::Shell => Choice::Shell,
                    };
                    break Ok(choice);
                }
                _ => {}
            }
        }
    };

    // Tear down the alternate rendering: clear what we drew and restore cursor.
    let _ = clear_menu(&mut stderr, &items);
    let _ = execute!(stderr, cursor::Show);
    let _ = terminal::disable_raw_mode();
    result
}

/// Total lines the menu occupies on screen:
/// header (1) + blank (1) + items + blank (1) + footer (1).
fn line_count(items: &[Item]) -> u16 {
    (items.len() as u16) + 4
}

fn clear_menu(w: &mut impl Write, items: &[Item]) -> io::Result<()> {
    // The cursor rests on the footer (last) line, so move up line_count-1 to
    // reach the header, then wipe everything below.
    execute!(
        w,
        cursor::MoveToColumn(0),
        cursor::MoveUp(line_count(items) - 1),
        Clear(ClearType::FromCursorDown),
    )
}

fn draw(
    w: &mut impl Write,
    sessions: &[Session],
    items: &[Item],
    selected: usize,
    version: &str,
    first: &mut bool,
) -> io::Result<()> {
    // Move back to the top of our block (after the first frame) and redraw.
    if *first {
        *first = false;
    } else {
        execute!(
            w,
            cursor::MoveToColumn(0),
            cursor::MoveUp(line_count(items) - 1),
        )?;
    }

    execute!(
        w,
        Clear(ClearType::FromCursorDown),
        SetForegroundColor(Color::Cyan),
        SetAttribute(Attribute::Bold),
        Print(format!("  tmosh {version}\r\n")),
        ResetColor,
        Print("\r\n"),
    )?;

    for (idx, item) in items.iter().enumerate() {
        let is_sel = idx == selected;
        let pointer = if is_sel { "›" } else { " " };

        if is_sel {
            execute!(
                w,
                SetForegroundColor(Color::Cyan),
                SetAttribute(Attribute::Bold)
            )?;
        }

        match item {
            Item::Session(i) => {
                let s = &sessions[*i];
                let label = format!(
                    "  {pointer} {name}  ({win}w, {act})\r\n",
                    name = s.name,
                    win = s.windows,
                    act = s.activity,
                );
                execute!(w, Print(label))?;
            }
            Item::NewSession => {
                execute!(w, Print(format!("  {pointer} + new session\r\n")))?;
            }
            Item::Shell => {
                execute!(w, Print(format!("  {pointer} shell (no tmux)\r\n")))?;
            }
        }

        if is_sel {
            execute!(w, ResetColor)?;
        }
    }

    // Footer ends WITHOUT a trailing newline so the cursor rests on this last
    // line; the next redraw moves up exactly line_count-1 to realign.
    execute!(
        w,
        Print("\r\n"),
        SetForegroundColor(Color::DarkGrey),
        Print("  ↑/↓ move · enter select · esc → shell"),
        ResetColor,
    )?;
    w.flush()
}
