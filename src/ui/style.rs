use anstyle::{AnsiColor, Style};
use clap::builder::Styles;

/// @agent The one palette for the whole CLI. Print painted text through
/// `anstream`, which strips the colour codes when the output is not a terminal,
/// so agents and pipes receive plain text.
#[derive(Clone, Copy)]
pub enum Tone {
    Good,
    Caution,
    Bad,
    Muted,
    Strong,
}

impl Tone {
    pub fn paint(self, text: &str) -> String {
        let style = self.style();
        format!("{style}{text}{style:#}")
    }

    pub fn table_color(self) -> Option<comfy_table::Color> {
        match self {
            Tone::Good => Some(comfy_table::Color::Green),
            Tone::Caution => Some(comfy_table::Color::Yellow),
            Tone::Bad => Some(comfy_table::Color::Red),
            Tone::Muted => Some(comfy_table::Color::DarkGrey),
            Tone::Strong => None,
        }
    }

    fn style(self) -> Style {
        match self {
            Tone::Good => AnsiColor::BrightGreen.on_default().bold(),
            Tone::Caution => AnsiColor::BrightYellow.on_default().bold(),
            Tone::Bad => AnsiColor::BrightRed.on_default().bold(),
            Tone::Muted => Style::new().dimmed(),
            Tone::Strong => Style::new().bold(),
        }
    }
}

pub fn help_styles() -> Styles {
    Styles::styled()
        .header(Tone::Strong.style().underline())
        .usage(Tone::Strong.style().underline())
        .literal(AnsiColor::Cyan.on_default().bold())
        .placeholder(AnsiColor::Cyan.on_default())
        .error(Tone::Bad.style())
        .valid(Tone::Good.style())
        .invalid(Tone::Caution.style())
}
