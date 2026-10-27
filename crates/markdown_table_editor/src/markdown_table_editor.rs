mod emit;
mod format;
mod modal;
mod parse;
#[cfg(test)]
mod tests;

pub use emit::emit;
pub use format::format_table_edits;
pub use modal::TableEditorModal;
pub use parse::{ParsedTable, parse_table_at, parse_tables};

use editor::Editor;
use gpui::{App, Context, Entity, Window};
use language::Buffer;
use parse::{ColumnAlign, TableData, TableStyle};
use project::Project;
use std::ops::Range;
use std::sync::Arc;
use text::ToPoint as _;
use workspace::Workspace;
use zed_actions::table::{EditTable, NewTable};

pub fn init(cx: &mut App) {
    cx.observe_new(|workspace: &mut Workspace, window, cx| {
        if window.is_none() {
            return;
        }
        workspace
            .register_action(|workspace, _: &EditTable, window, cx| {
                edit_table(workspace, window, cx);
            })
            .register_action(|workspace, _: &NewTable, window, cx| {
                new_table(workspace, window, cx);
            });
    })
    .detach();

    cx.observe_new(|project: &mut Project, _window, cx| {
        project.lsp_store().update(cx, |lsp_store, _| {
            lsp_store.register_buffer_save_transform(Arc::new(format_tables_on_save));
        });
    })
    .detach();
}

/// Rewrites every markdown table in the buffer when it is saved, padding rows
/// that are missing cells and aligning the columns.
fn format_tables_on_save(buffer: &Buffer) -> Option<Vec<(Range<usize>, String)>> {
    if buffer.language()?.name().as_ref() != "Markdown" {
        return None;
    }
    format_table_edits(&buffer.snapshot().text())
}

fn edit_table(workspace: &mut Workspace, window: &mut Window, cx: &mut Context<Workspace>) {
    let Some(editor) = workspace.active_item_as::<Editor>(cx) else {
        return;
    };

    let offset = editor.update(cx, |editor, cx| {
        let snapshot = editor.snapshot(window, cx);
        let cursor = editor.selections.newest_display(&snapshot).head();
        snapshot
            .buffer_snapshot()
            .point_to_offset(cursor.to_point(&snapshot))
            .0
    });
    let text = editor
        .read(cx)
        .buffer()
        .read(cx)
        .snapshot(cx)
        .text();
    let Some(parsed) = parse_table_at(&text, offset) else {
        return;
    };

    let style = parsed.style.clone();
    workspace.toggle_modal(window, cx, move |window, cx| {
        let on_apply = move |data: TableData, _window: &mut Window, cx: &mut App| {
            replace_table(&editor, parsed.range.clone(), emit(&data, &style), cx);
        };
        TableEditorModal::new(parsed.data, parsed.style, Box::new(on_apply), window, cx)
    });
}

fn new_table(workspace: &mut Workspace, window: &mut Window, cx: &mut Context<Workspace>) {
    let Some(editor) = workspace.active_item_as::<Editor>(cx) else {
        return;
    };

    let style = TableStyle {
        leading_pipe: true,
        trailing_pipe: true,
    };
    let modal_style = style.clone();
    let data = TableData {
        columns: vec![ColumnAlign::None, ColumnAlign::None],
        rows: vec![
            vec![String::new(), String::new()],
            vec![String::new(), String::new()],
        ],
    };
    let skeleton = emit(&data, &style);

    let table_range = editor.update(cx, |editor, cx| {
        let snapshot = editor.snapshot(window, cx);
        let cursor = editor.selections.newest_display(&snapshot).head();
        let offset = snapshot
            .buffer_snapshot()
            .point_to_offset(cursor.to_point(&snapshot))
            .0;
        let text = editor.buffer().read(cx).snapshot(cx).text();
        let prefix = if offset == 0 || text[..offset].ends_with('\n') {
            ""
        } else {
            "\n"
        };
        let suffix = if offset == text.len() || text[offset..].starts_with('\n') {
            ""
        } else {
            "\n"
        };
        let inserted = format!("{prefix}{skeleton}{suffix}");
        let start = offset + prefix.len();
        let table_range = start..start + skeleton.len();
        replace_table(&cx.entity(), offset..offset, inserted, cx);
        table_range
    });

    workspace.toggle_modal(window, cx, move |window, cx| {
        let on_apply = move |data: TableData, _window: &mut Window, cx: &mut App| {
            replace_table(&editor, table_range.clone(), emit(&data, &style), cx);
        };
        TableEditorModal::new(data, modal_style, Box::new(on_apply), window, cx)
    });
}

fn replace_table(editor: &Entity<Editor>, range: Range<usize>, text: String, cx: &mut App) {
    let buffer = editor
        .read(cx)
        .buffer()
        .read(cx)
        .as_singleton();
    if let Some(buffer) = buffer {
        buffer.update(cx, |buffer, cx| {
            buffer.edit([(range, text)], None, cx);
        });
    }
}
