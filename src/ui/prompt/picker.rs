use super::screen::{Key, Screen};
use super::{footer, heading, Answer};
use crate::ui::Tone;
use anyhow::Result;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Radio {
    Absent,
    Filled,
    Empty,
}

pub struct Row {
    pub label: String,
    pub note: String,
    pub radio: Radio,
}

/// A list the arrow keys move through. `ask` answers with the index of the chosen row.
pub struct Picker<'a> {
    pub title: &'a str,
    pub subtitle: &'a str,
    pub rows: Vec<Row>,
    pub cursor: usize,
    pub help: &'a str,
}

impl Picker<'_> {
    pub fn ask(mut self, screen: &mut Screen) -> Result<Answer<usize>> {
        let last = self.rows.len().saturating_sub(1);
        loop {
            screen.draw(&self.lines())?;
            match screen.read_key()? {
                Key::Up | Key::Char('k') => self.cursor = self.cursor.checked_sub(1).unwrap_or(last),
                Key::Down | Key::Char('j') => self.cursor = if self.cursor == last { 0 } else { self.cursor + 1 },
                Key::Enter | Key::Char(' ') => return Ok(Answer::Chosen(self.cursor)),
                Key::Escape | Key::Char('q') => return Ok(Answer::Back),
                Key::Interrupt => return Ok(Answer::Quit),
                _ => {}
            }
        }
    }

    fn lines(&self) -> Vec<String> {
        let label_width = self.rows.iter().map(|row| row.label.chars().count()).max().unwrap_or(0);
        let rows = self.rows.iter().enumerate().map(|(index, row)| {
            let under_cursor = index == self.cursor;
            let pointer = if under_cursor { Tone::Good.paint("❯") } else { " ".to_string() };
            let radio = match row.radio {
                Radio::Absent => String::new(),
                Radio::Filled => format!("{} ", Tone::Good.paint("●")),
                Radio::Empty => format!("{} ", Tone::Muted.paint("○")),
            };
            let label = format!("{:<label_width$}", row.label);
            let label = if under_cursor { Tone::Strong.paint(&label) } else { label };
            format!("{pointer} {radio}{label}   {}", Tone::Muted.paint(&row.note))
        });

        let mut lines = heading(self.title, self.subtitle);
        lines.extend(rows);
        lines.extend(footer(self.help));
        lines
    }
}
