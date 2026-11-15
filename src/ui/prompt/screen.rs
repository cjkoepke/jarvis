use anstream::{adapter::strip_str, AutoStream, ColorChoice};
use anyhow::{bail, Result};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::terminal::{self, ClearType};
use crossterm::{cursor, queue};
use std::io::{self, IsTerminal, Stdout, Write};

pub enum Key {
    Up,
    Down,
    Enter,
    Escape,
    Interrupt,
    Backspace,
    Char(char),
    Other,
}

/// A block of lines below the shell prompt that is redrawn in place and gone once dropped.
pub struct Screen {
    out: Stdout,
    lines_drawn: u16,
    colored: bool,
}

impl Screen {
    pub fn open() -> Result<Screen> {
        let out = io::stdout();
        if !io::stdin().is_terminal() || !out.is_terminal() {
            bail!("this is not an interactive terminal");
        }
        let colored = AutoStream::choice(&out) != ColorChoice::Never;
        terminal::enable_raw_mode()?;
        let mut screen = Screen {
            out,
            lines_drawn: 0,
            colored,
        };
        queue!(screen.out, cursor::Hide, terminal::DisableLineWrap)?;
        Ok(screen)
    }

    pub fn draw(&mut self, lines: &[String]) -> Result<()> {
        self.erase()?;
        for line in lines {
            if self.colored {
                write!(self.out, "{line}\r\n")?;
            } else {
                write!(self.out, "{}\r\n", strip_str(line))?;
            }
        }
        self.lines_drawn = u16::try_from(lines.len())?;
        Ok(self.out.flush()?)
    }

    pub fn read_key(&mut self) -> Result<Key> {
        let Event::Key(key) = event::read()? else {
            return Ok(Key::Other);
        };
        Ok(meaning_of(key))
    }

    fn erase(&mut self) -> io::Result<()> {
        if self.lines_drawn > 0 {
            queue!(self.out, cursor::MoveUp(self.lines_drawn))?;
        }
        self.lines_drawn = 0;
        queue!(self.out, cursor::MoveToColumn(0), terminal::Clear(ClearType::FromCursorDown))
    }
}

impl Drop for Screen {
    fn drop(&mut self) {
        self.erase().ok();
        queue!(self.out, terminal::EnableLineWrap, cursor::Show).ok();
        self.out.flush().ok();
        terminal::disable_raw_mode().ok();
    }
}

fn meaning_of(key: KeyEvent) -> Key {
    if key.kind == KeyEventKind::Release {
        return Key::Other;
    }
    let holds_control = key.modifiers.contains(KeyModifiers::CONTROL);
    match key.code {
        KeyCode::Char('c' | 'd') if holds_control => Key::Interrupt,
        KeyCode::Char(_) if holds_control => Key::Other,
        KeyCode::Char(character) => Key::Char(character),
        KeyCode::Up => Key::Up,
        KeyCode::Down => Key::Down,
        KeyCode::Enter => Key::Enter,
        KeyCode::Esc => Key::Escape,
        KeyCode::Backspace => Key::Backspace,
        _ => Key::Other,
    }
}
