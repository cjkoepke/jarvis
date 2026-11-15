mod picker;
mod screen;
mod text_input;

pub use picker::{Picker, Radio, Row};
pub use screen::Screen;
pub use text_input::TextInput;

use super::Tone;

pub enum Answer<T> {
    Chosen(T),
    Back,
    Quit,
}

impl<T> Answer<T> {
    pub fn map<U>(self, change: impl FnOnce(T) -> U) -> Answer<U> {
        match self {
            Answer::Chosen(chosen) => Answer::Chosen(change(chosen)),
            Answer::Back => Answer::Back,
            Answer::Quit => Answer::Quit,
        }
    }
}

fn heading(title: &str, subtitle: &str) -> Vec<String> {
    vec![Tone::Strong.paint(title), Tone::Muted.paint(subtitle), String::new()]
}

fn footer(help: &str) -> Vec<String> {
    vec![String::new(), Tone::Muted.paint(help)]
}
