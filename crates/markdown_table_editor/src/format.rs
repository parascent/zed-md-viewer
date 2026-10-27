//! Save-time normalization of every table in a document.

use crate::emit::emit;
use crate::parse::parse_tables;
use std::ops::Range;

/// Rewrites every table in `text` so columns are aligned and rows missing cells
/// are padded to the table's width. Returns the byte-range edits to apply, or
/// `None` when the document is already formatted.
pub fn format_table_edits(text: &str) -> Option<Vec<(Range<usize>, String)>> {
    let mut edits = Vec::new();
    for table in parse_tables(text) {
        if table.has_extra_cells {
            continue;
        }
        let formatted = emit(&table.data, &table.style)
            .lines()
            .map(|line| format!("{}{line}", table.indent))
            .collect::<Vec<_>>()
            .join("\n");
        if text[table.range.clone()] != formatted {
            edits.push((table.range, formatted));
        }
    }
    (!edits.is_empty()).then_some(edits)
}
