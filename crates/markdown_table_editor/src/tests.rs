use crate::{emit, format_table_edits, parse::ColumnAlign, parse_table_at};

const TABLE: &str = "\
# Notes

| Name | Qty |
| ---- | ---: |
| apple | 1 |
| pear | 22 |

after text";

fn offset_of(haystack: &str, needle: &str) -> usize {
    haystack
        .find(needle)
        .unwrap_or_else(|| panic!("fixture missing {needle:?}"))
}

fn format_document(text: &str) -> Option<String> {
    let edits = format_table_edits(text)?;
    let mut out = String::new();
    let mut last = 0;
    for (range, new_text) in edits {
        assert!(range.start >= last, "edits must be in order");
        out.push_str(&text[last..range.start]);
        out.push_str(&new_text);
        last = range.end;
    }
    out.push_str(&text[last..]);
    Some(out)
}

fn init_test_app(cx: &mut gpui::TestAppContext) {
    cx.update(|cx| {
        assets::Assets.load_test_fonts(cx);
        let store = settings::SettingsStore::test(cx);
        cx.set_global(store);
        theme_settings::init(theme::LoadThemes::JustBase, cx);
        release_channel::init(semver::Version::new(0, 0, 0), cx);
        editor::init(cx);
    });
}

#[test]
fn parses_table_from_body_cursor() {
    let at = offset_of(TABLE, "pear");
    let table = parse_table_at(TABLE, at).expect("table at cursor");
    assert_eq!(table.data.columns, vec![ColumnAlign::None, ColumnAlign::Right]);
    assert_eq!(table.data.rows.len(), 3);
    assert_eq!(table.data.rows[0], vec!["Name".to_string(), "Qty".to_string()]);
    assert_eq!(table.data.rows[2], vec!["pear".to_string(), "22".into()]);
    assert!(table.style.leading_pipe && table.style.trailing_pipe);
    // Range covers exactly the three table lines.
    assert_eq!(&TABLE[table.range.clone()], "| Name | Qty |\n| ---- | ---: |\n| apple | 1 |\n| pear | 22 |");
}

#[test]
fn parses_from_header_and_delimiter_cursors() {
    for needle in ["| Name | Qty |", "| ---- | ---: |"] {
        let at = offset_of(TABLE, needle);
        assert!(parse_table_at(TABLE, at).is_some(), "cursor on {needle:?}");
    }
}

#[test]
fn rejects_cursor_outside_table() {
    for needle in ["# Notes", "after text"] {
        let at = offset_of(TABLE, needle);
        assert!(parse_table_at(TABLE, at).is_none(), "cursor on {needle:?}");
    }
}

#[test]
fn rejects_tables_inside_code_fences() {
    let fenced = "text\n\n```rust\n| a | b |\n| - | - |\n| 1 | 2 |\n```\n";
    let at = offset_of(fenced, "| 1 | 2 |");
    assert!(parse_table_at(fenced, at).is_none());
}

#[test]
fn handles_escaped_pipes_and_ragged_rows() {
    let text = "a | b\n--- | ---\nc \\| d | e\ntoo | many | cells";
    let at = offset_of(text, "c \\| d");
    let table = parse_table_at(text, at).expect("table");
    assert_eq!(table.data.rows[1], vec!["c | d".to_string(), "e".to_string()]);
    // Extra cell is truncated, missing cells are padded.
    assert_eq!(table.data.rows[2], vec!["too".to_string(), "many".to_string()]);
    assert!(!table.style.leading_pipe && !table.style.trailing_pipe);
}

#[test]
fn rejects_pipe_line_without_delimiter_row() {
    let text = "| not a table |\n| still not |";
    let at = offset_of(text, "not a table");
    assert!(parse_table_at(text, at).is_none());
}

#[test]
fn emit_is_stable_and_aligned() {
    let table = parse_table_at(TABLE, offset_of(TABLE, "apple")).unwrap();
    let emitted = emit(&table.data, &table.style);
    assert_eq!(
        emitted,
        "\
| Name  | Qty |
| ----- | --: |
| apple | 1   |
| pear  | 22  |"
    );
    // Round-trip: the emitted form parses to the same data.
    let reparsed = parse_table_at(&emitted, offset_of(&emitted, "apple")).unwrap();
    assert_eq!(reparsed.data, table.data);
    assert_eq!(emit(&reparsed.data, &reparsed.style), emitted);
}

#[test]
fn emit_escapes_pipes_and_strips_newlines() {
    let data = crate::parse::TableData {
        columns: vec![ColumnAlign::Center, ColumnAlign::None],
        rows: vec![
            vec!["a|b".into(), "x".into()],
            vec!["line\nbreak".into(), "y".into()],
        ],
    };
    let style = crate::parse::TableStyle {
        leading_pipe: true,
        trailing_pipe: true,
    };
    let emitted = emit(&data, &style);
    let lines: Vec<&str> = emitted.split('\n').collect();
    assert_eq!(lines.len(), 3);
    // Header first, then the centered delimiter, then the body with the
    // newline stripped and the pipe escaped.
    assert!(lines[0].starts_with("| a\\|b"));
    assert!(lines[0].ends_with("| x   |"));
    assert!(lines[1].contains(":-") && lines[1].trim_end_matches(|c| c == '-' || c == ':').ends_with('|'));
    assert_eq!(lines[2], "| line break | y   |");
    // All lines are padded to the same width.
    let widths: Vec<usize> = lines.iter().map(|l| l.chars().count()).collect();
    assert!(widths.windows(2).all(|w| w[0] == w[1]));
    // Round-trip: emitted markdown parses back to the same data, with the
    // newline normalized to a space (stripping is intentional).
    let reparsed = parse_table_at(&emitted, emitted.find("line break").unwrap()).unwrap();
    assert_eq!(
        reparsed.data,
        crate::parse::TableData {
            columns: vec![ColumnAlign::Center, ColumnAlign::None],
            rows: vec![
                vec!["a|b".to_string(), "x".to_string()],
                vec!["line break".to_string(), "y".to_string()],
            ],
        }
    );
}

#[test]
fn emit_preserves_pipeless_style() {
    let text = "a | b\n--- | ---\n1 | 2";
    let table = parse_table_at(text, offset_of(text, "1 | 2")).unwrap();
    assert_eq!(emit(&table.data, &table.style), "a   | b  \n--- | ---\n1   | 2  ");
}

#[test]
fn parses_crlf_tables() {
    let text = "| a | b |\r\n| - | - |\r\n| 1 | 2 |";
    let at = offset_of(text, "1 | 2");
    let table = parse_table_at(text, at).expect("crlf table");
    assert_eq!(table.data.rows[1], vec!["1".to_string(), "2".to_string()]);
}

#[gpui::test]
async fn table_editor_modal_row_and_column_mutations(cx: &mut gpui::TestAppContext) {
    use crate::modal::TableEditorModal;
    use crate::parse::{ColumnAlign, TableData, TableStyle};
    use gpui::AppContext as _;

    init_test_app(cx);

    let data = TableData {
        columns: vec![ColumnAlign::None, ColumnAlign::None],
        rows: vec![
            vec!["a".to_string(), "b".to_string()],
            vec!["c".to_string(), "d".to_string()],
        ],
    };
    let style = TableStyle {
        leading_pipe: true,
        trailing_pipe: true,
    };
    let window = cx.add_window(|window, cx| {
        TableEditorModal::new(data, style, Box::new(|_, _, _| {}), window, cx)
    });

    window.update(cx, |modal, window, cx| {
        modal.add_row(window, cx);
        assert_eq!(modal.table_data().rows.len(), 3);
        modal.add_column(window, cx);
        assert_eq!(modal.table_data().columns.len(), 3);
        assert_eq!(modal.table_data().rows[0].len(), 3);

        modal.insert_row(0, window, cx);
        assert_eq!(modal.table_data().rows.len(), 4);
        assert_eq!(modal.table_data().rows[0], vec![String::new(); 3]);

        modal.delete_row(0, window, cx);
        assert_eq!(modal.table_data().rows.len(), 3);
        assert_eq!(modal.table_data().rows[0][0], "a");

        modal.delete_column(2, window, cx);
        assert_eq!(modal.table_data().columns.len(), 2);
        assert_eq!(modal.table_data().rows[1].len(), 2);

        // The last row and column can never be deleted.
        modal.delete_row(0, window, cx);
        modal.delete_row(0, window, cx);
        assert_eq!(modal.table_data().rows.len(), 1);
        modal.delete_column(0, window, cx);
        modal.delete_column(0, window, cx);
        assert_eq!(modal.table_data().columns.len(), 1);
    });
}

#[gpui::test]
async fn table_editor_modal_scrolls_horizontally_when_wider_than_the_modal(
    cx: &mut gpui::TestAppContext,
) {
    use crate::modal::TableEditorModal;
    use crate::parse::{ColumnAlign, TableData, TableStyle};
    use gpui::{AppContext as _, px, size};

    init_test_app(cx);

    // Ten columns are 1440px of fixed-width cells, far wider than the 46rem modal.
    let column_count = 10;
    let data = TableData {
        columns: vec![ColumnAlign::None; column_count],
        rows: vec![vec![String::new(); column_count]; 2],
    };
    let style = TableStyle {
        leading_pipe: true,
        trailing_pipe: true,
    };
    let window = cx.open_window(size(px(800.), px(600.)), |window, cx| {
        TableEditorModal::new(
            data,
            style.clone(),
            Box::new(|_, _, _| {}),
            window,
            cx,
        )
    });

    cx.update_window(window.into(), |_, window, cx| window.draw(cx).clear(cx))
        .unwrap();

    let max_offset = window
        .update(cx, |modal, _, _| modal.grid_scroll_handle().max_offset())
        .unwrap();
    assert!(
        max_offset.x > px(0.),
        "grid should scroll horizontally, max offset was {max_offset:?}"
    );
    assert_eq!(max_offset.y, px(0.), "two rows fit without vertical scrolling");

    // Tall tables scroll vertically inside the capped grid height.
    let data = TableData {
        columns: vec![ColumnAlign::None; 2],
        rows: vec![vec![String::new(); 2]; 25],
    };
    let window = cx.open_window(size(px(800.), px(600.)), |window, cx| {
        TableEditorModal::new(data, style.clone(), Box::new(|_, _, _| {}), window, cx)
    });
    cx.update_window(window.into(), |_, window, cx| window.draw(cx).clear(cx))
        .unwrap();
    let max_offset = window
        .update(cx, |modal, _, _| modal.grid_scroll_handle().max_offset())
        .unwrap();
    assert!(max_offset.y > px(0.), "tall grid should scroll vertically");

    // A table narrower than the modal must not become horizontally scrollable.
    let data = TableData {
        columns: vec![ColumnAlign::None; 2],
        rows: vec![vec![String::new(); 2]; 2],
    };
    let window = cx.open_window(size(px(800.), px(600.)), |window, cx| {
        TableEditorModal::new(data, style, Box::new(|_, _, _| {}), window, cx)
    });
    cx.update_window(window.into(), |_, window, cx| window.draw(cx).clear(cx))
        .unwrap();
    let max_offset = window
        .update(cx, |modal, _, _| modal.grid_scroll_handle().max_offset())
        .unwrap();
    assert_eq!(max_offset.x, px(0.));
}

#[test]
fn formats_ragged_table_and_pads_missing_cells() {
    let text = "\
# Notes

| Name | Qty |
| --- | ---: |
| apple | 1 |
| pear |

after text";
    let formatted = format_document(text).expect("table needs formatting");
    assert_eq!(
        formatted,
        "\
# Notes

| Name  | Qty |
| ----- | --: |
| apple | 1   |
| pear  |     |

after text"
    );
}

#[test]
fn format_is_idempotent() {
    let text = "\
| a | b |
| - | - |
| 1 | 2 |";
    let once = format_document(text).expect("needs formatting");
    assert_eq!(format_document(&once), None);
}

#[test]
fn format_skips_fenced_code_blocks() {
    let text = "\
```
| a | b |
| - | - |
```

| a | b |
| - | - |";
    let formatted = format_document(text).expect("only the real table changes");
    assert!(formatted.starts_with("```\n| a | b |\n| - | - |\n```"));
    assert!(formatted.ends_with("| a   | b   |\n| --- | --- |"));
}

#[test]
fn format_keeps_table_indentation() {
    let text = "- item\n\n  | a | b |\n  | - | - |\n  | 1 |\n";
    let formatted = format_document(text).expect("needs formatting");
    assert_eq!(
        formatted,
        "- item\n\n  | a   | b   |\n  | --- | --- |\n  | 1   |     |\n"
    );
}

#[test]
fn format_leaves_overwide_rows_alone() {
    let text = "| a | b |\n| - | - |\n| 1 | 2 | 3 |";
    assert_eq!(format_document(text), None);
}

#[test]
fn format_handles_header_only_tables() {
    let text = "| a | b |\n| - | - |";
    let formatted = format_document(text).expect("needs formatting");
    assert_eq!(formatted, "| a   | b   |\n| --- | --- |");
}

#[test]
fn formats_every_table_in_the_document() {
    let text = "\
| a | b |
| - | - |

middle

| c | d |
| - | - |";
    let formatted = format_document(text).expect("both tables need formatting");
    assert_eq!(
        formatted,
        "\
| a   | b   |
| --- | --- |

middle

| c   | d   |
| --- | --- |"
    );
}
