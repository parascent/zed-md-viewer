# zed-md-viewer

A personal fork of [Zed](https://github.com/zed-industries/zed) reduced to a
single-file Markdown editor: one file per window, Vim keybindings, and a
rendered preview that opens by default.

## What it does

- **One file per window.** No tabs, splits, side panels, project panel, AI, or
  telemetry. Launching another file replaces the current one.
- **Starts in viewing mode.** Markdown files open as the rendered preview.
  `ctrl-shift-v` toggles between the preview and the raw editor (macOS:
  `cmd-shift-v`).
- **Table editor.** With the cursor in a table, `ctrl-alt-t` opens a
  spreadsheet-style modal (`cmd-alt-t` on macOS): click row numbers or column
  letters to insert/delete, add rows and columns, then Apply. `ctrl-alt-shift-t`
  inserts a new 2x2 table and opens it.
- **Tables format on save.** Every table in a Markdown buffer is normalized
  before the file is written: ragged rows are padded with the missing cells,
  columns are aligned, and the delimiter row is rebuilt at matching width. Pipe
  style and indentation (tables nested in list items) are preserved; fenced code
  blocks and tables with over-wide rows are left untouched. Formatting is
  idempotent and needs no configuration.
- **Centered reading layout** with margins that shrink with the window; toggle
  with `alt-z`.

## Install (Linux, user-local)

```sh
cargo build --release -p zed
install -m 755 target/release/zed-md-viewer ~/.local/bin/zed-md-viewer

sed "s|@BIN@|$HOME/.local/bin/zed-md-viewer|g" \
  crates/zed/resources/zed-md-viewer.desktop \
  > ~/.local/share/applications/zed-md-viewer.desktop
update-desktop-database ~/.local/share/applications
```

The desktop entry claims Markdown only
(`MimeType=text/markdown;text/x-markdown;`), so it is offered as a handler for
`.md`, `.markdown` and `.mkd` files and never for other text files. Use
`xdg-mime default zed-md-viewer.desktop text/markdown` to make it the default
handler if it is not already.

Settings live in `~/.config/zed-md-viewer/settings.json`; data (database, logs)
in `~/.local/share/zed-md-viewer/`.

## Development

```sh
./dev.sh            # debug build + the test fixture
./dev.sh notes.md   # debug build + your file(s)
./dev.sh --release  # release build
```

Debug builds read `assets/settings/*` and `assets/keymaps/*` from the checkout at
runtime, so settings and keymap edits apply on relaunch without rebuilding.

Run the crate's tests with `cargo test -p markdown_table_editor`. Some upstream
`markdown_preview` tests fail by design in this fork because they exercise tabs
and split panes, which have been removed.

## Key bindings

| Action | Linux / Windows | macOS |
| --- | --- | --- |
| Return to editor / reopen preview | `ctrl-shift-v` | `cmd-shift-v` |
| Edit table at cursor | `ctrl-alt-t` | `cmd-alt-t` |
| Insert new table | `ctrl-alt-shift-t` | `cmd-alt-shift-t` |
| Toggle centered layout | `alt-z` | `alt-z` |

## Licensing

Zed is licensed under the GNU General Public License v3.0 or later; third-party
components carry their own licenses. See [LICENSE-GPL](LICENSE-GPL) and
[LICENSE-APACHE](LICENSE-APACHE).
