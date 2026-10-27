//! Emits a [`TableData`] back to GFM markdown with columns padded to equal
//! width, so the source diff is stable and readable.

use crate::parse::{ColumnAlign, TableData, TableStyle};

pub fn emit(data: &TableData, style: &TableStyle) -> String {
    let column_count = data.columns.len();
    // Escape before measuring so `\|` sequences count towards the column width.
    let rows: Vec<Vec<String>> = data
        .rows
        .iter()
        .map(|row| {
            row.iter()
                .take(column_count)
                .map(|cell| escape_cell(cell))
                .chain(std::iter::repeat(String::new()))
                .take(column_count)
                .collect()
        })
        .collect();

    let mut widths = vec![0usize; column_count];
    for row in &rows {
        for (col, cell) in row.iter().enumerate() {
            widths[col] = widths[col].max(display_width(cell));
        }
    }
    // The delimiter row needs at least three dashes per column; widen the data
    // columns to match so every emitted line has the same length.
    for width in &mut widths {
        *width = (*width).max(3);
    }

    let mut lines = Vec::with_capacity(rows.len() + 1);
    for (i, row) in rows.iter().enumerate() {
        lines.push(data_row(row, &widths, style));
        // The delimiter row always follows the header, including in tables that
        // have no body rows yet.
        if i == 0 {
            lines.push(delimiter_row(&widths, &data.columns, style));
        }
    }
    lines.join("\n")
}

fn data_row(row: &[String], widths: &[usize], style: &TableStyle) -> String {
    let cells: Vec<String> = row
        .iter()
        .zip(widths.iter())
        .map(|(cell, &width)| format!("{cell}{}", " ".repeat(width - display_width(cell))))
        .collect();
    let mut out = String::new();
    if style.leading_pipe {
        out.push_str("| ");
        out.push_str(&cells.join(" | "));
        out.push_str(" |");
    } else {
        out.push_str(&cells.join(" | "));
        if style.trailing_pipe {
            out.push_str(" |");
        }
    }
    out
}

fn delimiter_row(widths: &[usize], columns: &[ColumnAlign], style: &TableStyle) -> String {
    let cells: Vec<String> = widths
        .iter()
        .enumerate()
        .map(|(col, &width)| {
            let dashes = width.max(3);
            let inner = dashes.saturating_sub(2).max(1);
            match columns.get(col).copied().unwrap_or(ColumnAlign::None) {
                ColumnAlign::Left => format!(":{}", "-".repeat(dashes - 1)),
                ColumnAlign::Right => format!("{}:", "-".repeat(dashes - 1)),
                ColumnAlign::Center => format!(":{}:", "-".repeat(inner)),
                ColumnAlign::None => "-".repeat(dashes),
            }
        })
        .collect();
    if style.leading_pipe {
        format!("| {} |", cells.join(" | "))
    } else if style.trailing_pipe {
        format!("{} |", cells.join(" | "))
    } else {
        cells.join(" | ")
    }
}

fn escape_cell(cell: &str) -> String {
    let mut out = String::with_capacity(cell.len());
    for c in cell.chars() {
        match c {
            '|' => out.push_str("\\|"),
            '\n' | '\r' | '\t' => out.push(' '),
            _ => out.push(c),
        }
    }
    out
}

fn display_width(cell: &str) -> usize {
    cell.chars().count()
}
