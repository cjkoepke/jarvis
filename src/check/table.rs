use super::{Check, Status};
use crate::ui::Tone;
use comfy_table::{Attribute, Cell, ContentArrangement, ContentLineStyle, LineStyle, Table, TableStyle};

const COLUMNS: [&str; 4] = ["Area", "Check", "Status", "Detail"];

const ROUNDED_WITHOUT_COLUMN_LINES: TableStyle = TableStyle::new()
    .top_border(LineStyle::new('╭', '─', '─', '╮'))
    .header_lines(ContentLineStyle::new('│', ' ', '│'))
    .header_separator(LineStyle::new('├', '─', '─', '┤'))
    .content_lines(ContentLineStyle::new('│', ' ', '│'))
    .bottom_border(LineStyle::new('╰', '─', '─', '╯'));

pub fn render(checks: &[Check]) -> String {
    let mut table = Table::new();
    table.load_style(ROUNDED_WITHOUT_COLUMN_LINES);
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(COLUMNS.map(|title| Cell::new(title).add_attribute(Attribute::Bold)));

    let mut previous_area = None;
    for check in checks {
        let starts_new_area = previous_area != Some(&check.area);
        previous_area = Some(&check.area);
        table.add_row([
            Cell::new(if starts_new_area { check.area.as_str() } else { "" }).add_attribute(Attribute::Bold),
            Cell::new(&check.name),
            toned(Cell::new(check.status.label()), check.status.tone()),
            detail_cell(check),
        ]);
    }
    table.to_string()
}

fn detail_cell(check: &Check) -> Cell {
    let detail = Cell::new(&check.detail);
    match check.status {
        Status::Ok => toned(detail, Tone::Muted),
        Status::Warn | Status::Fail => detail,
    }
}

fn toned(cell: Cell, tone: Tone) -> Cell {
    match tone.table_color() {
        Some(color) => cell.fg(color),
        None => cell,
    }
}
