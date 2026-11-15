use super::screen::{Key, Screen};
use super::{footer, heading, Answer};
use crate::ui::Tone;
use anyhow::Result;

const HELP: &str = "enter save · esc back";

/// One line of typed text. An empty line answers `Back`.
pub struct TextInput<'a> {
    pub title: &'a str,
    pub subtitle: &'a str,
    pub text: String,
}

impl TextInput<'_> {
    pub fn ask(mut self, screen: &mut Screen) -> Result<Answer<String>> {
        loop {
            screen.draw(&self.lines())?;
            match screen.read_key()? {
                Key::Char(character) => self.text.push(character),
                Key::Backspace => drop(self.text.pop()),
                Key::Enter => {
                    let text = self.text.trim();
                    return Ok(if text.is_empty() {
                        Answer::Back
                    } else {
                        Answer::Chosen(text.to_string())
                    });
                }
                Key::Escape => return Ok(Answer::Back),
                Key::Interrupt => return Ok(Answer::Quit),
                _ => {}
            }
        }
    }

    fn lines(&self) -> Vec<String> {
        let mut lines = heading(self.title, self.subtitle);
        lines.push(format!("{} {}{}", Tone::Good.paint("❯"), self.text, Tone::Muted.paint("█")));
        lines.extend(footer(HELP));
        lines
    }
}
