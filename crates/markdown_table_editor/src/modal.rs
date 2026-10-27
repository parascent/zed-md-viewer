use crate::parse::{ColumnAlign, TableData, TableStyle};
use gpui::{Focusable as _, WeakEntity};
use menu;
use ui::{
    ActiveTheme as _, AlertModal, Button, ButtonCommon as _, ButtonStyle, Color, ContextMenu,
    Icon, IconName, IconSize, Label, LabelSize, PopoverMenu, ScrollAxes, Scrollbars, Tooltip,
    WithScrollbar as _,
    prelude::*,
};
use ui_input::InputField;
use workspace::{DismissDecision, ModalView};

type ApplyFn = Box<dyn FnOnce(TableData, &mut gpui::Window, &mut gpui::App) + 'static>;

const CELL_WIDTH: gpui::Pixels = px(140.);
const HEADER_WIDTH: gpui::Pixels = px(32.);
const ROW_HEIGHT: gpui::Pixels = px(28.);
const ADD_COLUMN_WIDTH: gpui::Pixels = px(20.);

/// Spreadsheet-style editor for a markdown table.
///
/// Opens a modal with one input field per cell. Structure is edited in the grid
/// itself, Excel-style: row numbers and column letters open insert/delete
/// menus, a `+` square adds a column, and a `+ Add Row` bar appends a row.
/// Apply syncs every field back into the table and hands it to the `on_apply`
/// callback, which owns the buffer write.
pub struct TableEditorModal {
    data: TableData,
    style: TableStyle,
    cells: Vec<Vec<gpui::Entity<InputField>>>,
    original_rows: Vec<Vec<String>>,
    on_apply: Option<ApplyFn>,
    force_dismiss: bool,
    focus_handle: gpui::FocusHandle,
    grid_scroll_handle: gpui::ScrollHandle,
}

impl TableEditorModal {
    pub fn new(
        data: TableData,
        style: TableStyle,
        on_apply: ApplyFn,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> Self {
        let original_rows = data.rows.clone();
        let mut this = Self {
            data,
            style,
            cells: Vec::new(),
            original_rows,
            on_apply: Some(on_apply),
            force_dismiss: false,
            focus_handle: cx.focus_handle(),
            grid_scroll_handle: gpui::ScrollHandle::new(),
        };
        this.rebuild_cells(window, cx);
        this
    }

    pub fn table_data(&self) -> &TableData {
        &self.data
    }

    pub fn table_style(&self) -> &TableStyle {
        &self.style
    }

    #[cfg(test)]
    pub(crate) fn grid_scroll_handle(&self) -> &gpui::ScrollHandle {
        &self.grid_scroll_handle
    }

    fn rebuild_cells(&mut self, window: &mut gpui::Window, cx: &mut gpui::Context<Self>) {
        self.cells = self
            .data
            .rows
            .iter()
            .map(|row| {
                row.iter()
                    .map(|text| {
                        let text = text.clone();
                        cx.new(|cx| {
                            let field = InputField::new(window, cx, "");
                            field.set_text(&text, window, cx);
                            field
                        })
                    })
                    .collect()
            })
            .collect();
    }

    fn sync_from_cells(&mut self, cx: &gpui::App) {
        for (row, cells) in self.cells.iter().enumerate() {
            for (col, cell) in cells.iter().enumerate() {
                if let Some(slot) = self.data.rows.get_mut(row).and_then(|r| r.get_mut(col)) {
                    *slot = cell.read(cx).text(cx);
                }
            }
        }
    }

    fn is_dirty(&mut self, cx: &mut gpui::Context<Self>) -> bool {
        self.sync_from_cells(cx);
        self.data.rows != self.original_rows
    }

    fn active_cell(&self, window: &gpui::Window, cx: &gpui::App) -> Option<(usize, usize)> {
        self.cells
            .iter()
            .enumerate()
            .flat_map(|(row, cells)| {
                cells.iter().enumerate().map(move |(col, cell)| (row, col, cell))
            })
            .find(|(_, _, cell)| {
                cell.read(cx)
                    .focus_handle(cx)
                    .contains_focused(window, cx)
            })
            .map(|(row, col, _)| (row, col))
    }

    pub(crate) fn insert_row(&mut self, at: usize, window: &mut gpui::Window, cx: &mut gpui::Context<Self>) {
        self.sync_from_cells(cx);
        let at = at.min(self.data.rows.len());
        self.data
            .rows
            .insert(at, vec![String::new(); self.data.columns.len()]);
        self.rebuild_cells(window, cx);
        cx.notify();
    }

    pub(crate) fn insert_column(
        &mut self,
        at: usize,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) {
        self.sync_from_cells(cx);
        let at = at.min(self.data.columns.len());
        self.data.columns.insert(at, ColumnAlign::None);
        for row in &mut self.data.rows {
            row.insert(at, String::new());
        }
        self.rebuild_cells(window, cx);
        cx.notify();
    }

    pub(crate) fn delete_row(&mut self, at: usize, window: &mut gpui::Window, cx: &mut gpui::Context<Self>) {
        if self.data.rows.len() <= 1 {
            return;
        }
        self.sync_from_cells(cx);
        if at < self.data.rows.len() {
            self.data.rows.remove(at);
        }
        self.rebuild_cells(window, cx);
        cx.notify();
    }

    pub(crate) fn delete_column(
        &mut self,
        at: usize,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) {
        if self.data.columns.len() <= 1 {
            return;
        }
        self.sync_from_cells(cx);
        if at < self.data.columns.len() {
            self.data.columns.remove(at);
            for row in &mut self.data.rows {
                if row.len() > at {
                    row.remove(at);
                }
            }
        }
        self.rebuild_cells(window, cx);
        cx.notify();
    }

    pub(crate) fn add_row(&mut self, window: &mut gpui::Window, cx: &mut gpui::Context<Self>) {
        let at = self.data.rows.len();
        self.insert_row(at, window, cx);
    }

    pub(crate) fn add_column(&mut self, window: &mut gpui::Window, cx: &mut gpui::Context<Self>) {
        let at = self.data.columns.len();
        self.insert_column(at, window, cx);
    }

    fn apply(&mut self, window: &mut gpui::Window, cx: &mut gpui::Context<Self>) {
        self.sync_from_cells(cx);
        if let Some(on_apply) = self.on_apply.take() {
            on_apply(self.data.clone(), window, cx);
        }
        self.force_dismiss = true;
        cx.emit(gpui::DismissEvent);
    }

    fn force_dismiss(&mut self, cx: &mut gpui::Context<Self>) {
        self.force_dismiss = true;
        cx.emit(gpui::DismissEvent);
    }
}

impl gpui::Focusable for TableEditorModal {
    fn focus_handle(&self, _cx: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl gpui::EventEmitter<gpui::DismissEvent> for TableEditorModal {}

impl ModalView for TableEditorModal {
    fn fade_out_background(&self) -> bool {
        true
    }

    fn on_before_dismiss(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> DismissDecision {
        if self.force_dismiss || !self.is_dirty(cx) {
            DismissDecision::Dismiss(true)
        } else {
            DismissDecision::Dismiss(false)
        }
    }
}

/// A small header square (column letter or row number) that opens an
/// insert/delete menu when clicked.
fn header_menu(
    id: impl Into<gpui::ElementId>,
    label: SharedString,
    width: gpui::Pixels,
    kind: HeaderKind,
    index: usize,
    this: &WeakEntity<TableEditorModal>,
) -> PopoverMenu<ContextMenu> {
    let this = this.clone();
    PopoverMenu::new(id)
        .trigger(
            Button::new(("header", index), label)
                .style(ButtonStyle::Subtle)
                .label_size(LabelSize::XSmall),
        )
        .menu(move |window, cx| {
            // The menu closure is `Fn`, so clone the weak handle per invocation
            // instead of moving it into the one-shot entry handlers.
            let this = this.clone();
            match kind {
                HeaderKind::Row => Some(ContextMenu::build(window, cx, move |menu, _, _| {
                    let above = this.clone();
                    let below = this.clone();
                    let delete = this.clone();
                    let above_index = index;
                    let below_index = index + 1;
                    let delete_index = index;
                    menu.entry("Insert Row Above", None, move |window, cx| {
                        _ = above.update(cx, |modal, cx| modal.insert_row(above_index, window, cx));
                    })
                    .entry("Insert Row Below", None, move |window, cx| {
                        _ = below.update(cx, |modal, cx| modal.insert_row(below_index, window, cx));
                    })
                    .separator()
                    .entry("Delete Row", None, move |window, cx| {
                        _ = delete.update(cx, |modal, cx| modal.delete_row(delete_index, window, cx));
                    })
                })),
                HeaderKind::Column => Some(ContextMenu::build(window, cx, move |menu, _, _| {
                    let left = this.clone();
                    let right = this.clone();
                    let delete = this.clone();
                    let left_index = index;
                    let right_index = index + 1;
                    let delete_index = index;
                    menu.entry("Insert Column Left", None, move |window, cx| {
                        _ = left.update(cx, |modal, cx| modal.insert_column(left_index, window, cx));
                    })
                    .entry("Insert Column Right", None, move |window, cx| {
                        _ = right.update(cx, |modal, cx| modal.insert_column(right_index, window, cx));
                    })
                    .separator()
                    .entry("Delete Column", None, move |window, cx| {
                        _ = delete.update(cx, |modal, cx| {
                            modal.delete_column(delete_index, window, cx)
                        });
                    })
                })),
            }
        })
}

enum HeaderKind {
    Row,
    Column,
}

impl gpui::Render for TableEditorModal {
    fn render(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        let column_count = self.data.columns.len();
        let active = self.active_cell(window, cx);
        let border = cx.theme().colors().border_variant;
        let panel = cx.theme().colors().editor_background;
        let this = cx.entity().downgrade();

        let column_letters = ['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J'];
        let mut column_header = h_flex().child(div().w(HEADER_WIDTH).h(px(20.)).flex_none());
        for col in 0..column_count {
            let letter = column_letters
                .get(col)
                .map_or_else(|| col.to_string(), |l| l.to_string());
            column_header = column_header.child(
                div()
                    .w(CELL_WIDTH)
                    .h(px(20.))
                    .flex_none()
                    .border_1()
                    .border_color(border)
                    .bg(panel)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(header_menu(
                                ("col-menu", col),
                                letter.into(),
                                CELL_WIDTH,
                                HeaderKind::Column,
                                col,
                                &this,
                            )),
                    ),
            );
        }
        let column_header = column_header.child(
            div()
                .w(ADD_COLUMN_WIDTH)
                .h(px(20.))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    Button::new("add-column", "+")
                        .style(ButtonStyle::Subtle)
                        .label_size(LabelSize::XSmall)
                        .tooltip(Tooltip::text("Add Column"))
                        .on_click(cx.listener(|this, _, window, cx| this.add_column(window, cx))),
                ),
        );

        let mut body = v_flex();
        for (row_index, row_cells) in self.cells.iter().enumerate() {
            let mut row = h_flex().child(
                div()
                    .w(HEADER_WIDTH)
                    .h(ROW_HEIGHT)
                    .flex_none()
                    .border_1()
                    .border_color(border)
                    .bg(panel)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(header_menu(
                        ("row-menu", row_index),
                        (row_index + 1).to_string().into(),
                        HEADER_WIDTH,
                        HeaderKind::Row,
                        row_index,
                        &this,
                    )),
            );
            for cell in row_cells {
                row = row.child(
                    div()
                        .w(CELL_WIDTH)
                        .h(ROW_HEIGHT)
                        .flex_none()
                        .border_1()
                        .border_color(border)
                        .child(cell.clone()),
                );
            }
            body = body.child(row);
        }
        let add_row_bar = h_flex()
            .child(
                Button::new("add-row", "Add Row")
                    .style(ButtonStyle::Outlined)
                    .label_size(LabelSize::Small)
                    .start_icon(Icon::new(IconName::Plus).size(IconSize::Small))
                    .on_click(cx.listener(|this, _, window, cx| this.add_row(window, cx))),
            )
            .id("add-row-bar");

        let grid = v_flex()
            // A scroll container's content size is measured from its direct
            // child's layout bounds, so the grid must be given the full table
            // width; stretching to the viewport would hide the overflow.
            .w_full()
            .min_w(HEADER_WIDTH + CELL_WIDTH * column_count as f32 + ADD_COLUMN_WIDTH)
            .child(column_header)
            .child(body)
            .child(add_row_bar);

        AlertModal::new("table-editor-modal")
            .title("Edit Table")
            .width(rems(46.))
            .key_context("TableEditor")
            .track_focus(&self.focus_handle(cx))
            .on_action(
                cx.listener(|this, _: &menu::Confirm, window, cx| this.apply(window, cx)),
            )
            .child(
                // Scrollbars must be attached to a non-scrolling wrapper: a
                // scrollbar that is a child of the scrolling element is painted
                // in scrolled coordinates and slides out of view.
                div()
                    .id("table-grid-container")
                    .w_full()
                    .min_w_0()
                    .child(
                        div()
                            .id("table-grid")
                            .w_full()
                            .min_w_0()
                            .min_h_0()
                            .max_h(rems(26.))
                            .overflow_scroll()
                            .track_scroll(&self.grid_scroll_handle)
                            .child(grid),
                    )
                    // Added after the grid so the scrollbars paint on top of it.
                    .custom_scrollbars(
                        Scrollbars::always_visible(ScrollAxes::Both)
                            // Keyed per modal so a reopened modal doesn't inherit
                            // the previous modal's scroll handle.
                            .id(("table-grid-scrollbars", cx.entity_id()))
                            .tracked_scroll_handle(&self.grid_scroll_handle),
                        window,
                        cx,
                    ),
            )
            .footer(
                h_flex()
                    .p_3()
                    .justify_between()
                    .child(
                        Label::new("Tab moves between cells · click a row number or column letter to insert or delete")
                            .size(LabelSize::Small)
                            .color(Color::Muted),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                Button::new("cancel", "Cancel")
                                    .color(Color::Muted)
                                    .on_click(cx.listener(|this, _, _, cx| this.force_dismiss(cx))),
                            )
                            .child(
                                Button::new("apply", "Apply")
                                    .when(active.is_none(), |b| b.color(Color::Muted))
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.apply(window, cx)
                                    })),
                            ),
                    ),
            )
    }
}
