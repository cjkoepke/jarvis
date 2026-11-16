use super::screen::{Key, Screen};
use super::{footer, heading, row_line, Answer};
use crate::ui::Tone;
use anyhow::Result;

const HELP: &str = "↑↓ move · space check · enter confirm · esc leave";

pub struct Item {
    pub label: String,
    pub note: String,
    pub checked: bool,
}

/// A list where space checks any number of items. `ask` answers with the indexes of the
/// checked ones.
pub struct Checklist<'a> {
    pub title: &'a str,
    pub subtitle: &'a str,
    pub items: Vec<Item>,
}

impl Checklist<'_> {
    pub fn ask(mut self, screen: &mut Screen) -> Result<Answer<Vec<usize>>> {
        let last = self.items.len().saturating_sub(1);
        let mut cursor = 0;
        loop {
            screen.draw(&self.lines(cursor))?;
            match screen.read_key()? {
                Key::Up | Key::Char('k') => cursor = cursor.checked_sub(1).unwrap_or(last),
                Key::Down | Key::Char('j') => cursor = if cursor == last { 0 } else { cursor + 1 },
                Key::Char(' ') => {
                    if let Some(item) = self.items.get_mut(cursor) {
                        item.checked = !item.checked;
                    }
                }
                Key::Enter => return Ok(Answer::Chosen(self.checked())),
                Key::Escape | Key::Char('q') => return Ok(Answer::Back),
                Key::Interrupt => return Ok(Answer::Quit),
                _ => {}
            }
        }
    }

    fn checked(&self) -> Vec<usize> {
        let indexed = self.items.iter().enumerate();
        indexed.filter(|(_, item)| item.checked).map(|(index, _)| index).collect()
    }

    fn lines(&self, cursor: usize) -> Vec<String> {
        let label_width = self.items.iter().map(|item| item.label.chars().count()).max().unwrap_or(0);
        let items = self.items.iter().enumerate().map(|(index, item)| {
            let checkbox = if item.checked {
                format!("{} ", Tone::Good.paint("◉"))
            } else {
                format!("{} ", Tone::Muted.paint("○"))
            };
            row_line(index == cursor, &checkbox, &format!("{:<label_width$}", item.label), &item.note)
        });

        let mut lines = heading(self.title, self.subtitle);
        lines.extend(items);
        lines.extend(footer(HELP));
        lines
    }
}
