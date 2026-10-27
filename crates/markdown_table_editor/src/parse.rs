//! Line-oriented parser for GFM markdown tables.
//!
//! GFM tables are strictly line-based, so parsing the source lines directly
//! gives exact byte ranges and preserves the author's pipe style — something
//! event-based parsers make awkward. `crates/markdown` (pulldown-cmark)
//! remains the reference for what counts as a table; the unit tests keep the
//! two honest.

use std::ops::Range;

/// Column alignment taken from the `|:---:|` delimiter row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColumnAlign {
    None,
    Left,
    Center,
    Right,
}

/// In-memory table snapshot. `rows[0]` is the header row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TableData {
    pub columns: Vec<ColumnAlign>,
    pub rows: Vec<Vec<String>>,
}

/// The pipe style of the source table, reproduced when emitting.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TableStyle {
    pub leading_pipe: bool,
    pub trailing_pipe: bool,
}

pub struct ParsedTable {
    /// Byte range of the table in the source, covering whole lines but not the
    /// trailing newline of the last line.
    pub range: Range<usize>,
    pub data: TableData,
    pub style: TableStyle,
    /// Leading whitespace of the header line, so a rewrite keeps the table
    /// nested where the author put it (e.g. inside a list item).
    pub indent: String,
    /// True when a row has more cells than the delimiter row. Rewriting such a
    /// table would drop the extra cells, so formatting leaves it alone.
    pub has_extra_cells: bool,
}

/// Returns the markdown table containing `offset`, if any.
///
/// `offset` is a byte offset into `text`. Lines inside fenced code blocks are
/// never treated as tables.
pub fn parse_table_at(text: &str, offset: usize) -> Option<ParsedTable> {
    parse_tables(text)
        .into_iter()
        .find(|table| offset >= table.range.start && offset <= table.range.end)
}

/// Returns every markdown table in `text`, in source order.
///
/// A table is a maximal run of non-blank, pipe-containing lines whose second
/// line is the delimiter row. Lines inside fenced code blocks are skipped.
pub fn parse_tables(text: &str) -> Vec<ParsedTable> {
    let (lines, fences) = index_lines(text);
    let view = |idx: usize| &text[lines[idx].clone()];
    let in_fence = |idx: usize| fences[idx];

    let mut tables = Vec::new();
    let mut start = 0;
    while start + 1 < lines.len() {
        if in_fence(start)
            || in_fence(start + 1)
            || !contains_unescaped_pipe(view(start))
            || view(start).trim().is_empty()
            || !is_delimiter_row(view(start + 1))
        {
            start += 1;
            continue;
        }

        let mut end = start + 1;
        while end + 1 < lines.len()
            && !in_fence(end + 1)
            && contains_unescaped_pipe(view(end + 1))
            && !view(end + 1).trim().is_empty()
        {
            end += 1;
        }

        // `start` is the header, `start + 1` the delimiter row; the body follows.
        let header_text = view(start);
        let columns: Vec<ColumnAlign> = split_row(view(start + 1))
            .iter()
            .map(|cell| align_of(cell))
            .collect();
        let mut has_extra_cells = false;
        let mut rows = Vec::with_capacity(end - start);
        for line in std::iter::once(start).chain(start + 2..=end) {
            let mut row = split_row(view(line));
            has_extra_cells |= row.len() > columns.len();
            row.resize(columns.len(), String::new());
            rows.push(row);
        }

        let indent_len = header_text.len() - header_text.trim_start().len();
        tables.push(ParsedTable {
            range: lines[start].start..lines[end].end,
            data: TableData { columns, rows },
            style: TableStyle {
                leading_pipe: header_text.trim_start().starts_with('|'),
                trailing_pipe: header_text.trim_end().ends_with('|'),
            },
            indent: header_text[..indent_len].to_string(),
            has_extra_cells,
        });
        start = end + 1;
    }
    tables
}

/// Splits one table row into unescaped, trimmed cells. Outer pipes are dropped,
/// mirroring the `| a | b |`, `a | b |`, and `a | b` shapes GFM accepts.
fn split_row(line: &str) -> Vec<String> {
    let trimmed = line.trim().trim_matches('\r');
    let mut cells = Vec::new();
    let mut current = String::new();
    let mut chars = trimmed.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                if chars.peek() == Some(&'|') {
                    current.push('|');
                    chars.next();
                } else {
                    current.push('\\');
                }
            }
            '|' => {
                cells.push(current.trim().to_string());
                current = String::new();
            }
            _ => current.push(c),
        }
    }
    cells.push(current.trim().to_string());

    // Drop the outer empty cells produced by leading/trailing pipes, but keep
    // genuinely empty first/last cells when the row has no outer pipes.
    if trimmed.starts_with('|') {
        cells.remove(0);
    }
    if trimmed.ends_with('|') && !trimmed.ends_with("\\|") {
        cells.pop();
    }
    cells
}

fn is_delimiter_row(line: &str) -> bool {
    let cells = split_row(line);
    !cells.is_empty()
        && cells.iter().all(|cell| {
            let dashes = cell.trim_matches(':');
            !dashes.is_empty() && dashes.chars().all(|c| c == '-')
        })
}

fn align_of(cell: &str) -> ColumnAlign {
    match (cell.starts_with(':'), cell.ends_with(':')) {
        (true, true) => ColumnAlign::Center,
        (true, false) => ColumnAlign::Left,
        (false, true) => ColumnAlign::Right,
        (false, false) => ColumnAlign::None,
    }
}

fn contains_unescaped_pipe(line: &str) -> bool {
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                chars.next();
            }
            '|' => return true,
            _ => {}
        }
    }
    false
}

/// Returns one range per line (excluding its `\n`) and whether each line is
/// inside — or is the marker of — a fenced code block.
fn index_lines(text: &str) -> (Vec<Range<usize>>, Vec<bool>) {
    let mut lines = Vec::new();
    let mut fences = Vec::new();
    let mut in_fence = false;
    let mut start = 0;
    for (i, c) in text.char_indices() {
        if c == '\n' {
            let line = &text[start..i];
            lines.push(start..i);
            if is_fence_marker(line) {
                in_fence = !in_fence;
            }
            fences.push(is_fence_marker(line) || in_fence);
            start = i + 1;
        }
    }
    let line = &text[start..];
    lines.push(start..text.len());
    fences.push(is_fence_marker(line) || in_fence);
    (lines, fences)
}

fn is_fence_marker(line: &str) -> bool {
    let trimmed = line.trim();
    let backticks = trimmed.chars().take_while(|&c| c == '`').count();
    let tildes = trimmed.chars().take_while(|&c| c == '~').count();
    // An info string is allowed after the fence chars (e.g. ```rust).
    backticks >= 3 || tildes >= 3
}
