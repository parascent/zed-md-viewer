#!/usr/bin/env bash
# Installs the zed-md-viewer binary and registers it as a Markdown-only
# application for the current user. No root required.
set -euo pipefail

cd "$(dirname "$0")"

BIN=${1:-target/release/zed-md-viewer}
if [[ ! -x $BIN ]]; then
    echo "error: $BIN not found. Build it first: cargo build --release -p zed" >&2
    exit 1
fi

install -Dm755 "$BIN" "$HOME/.local/bin/zed-md-viewer"
install -Dm644 crates/zed/resources/app-icon.png \
    "$HOME/.local/share/icons/hicolor/512x512/apps/zed-md-viewer.png"

# First install: carry over settings/keymap/themes from the dev checkout's
# config dir so the installed app starts with the same preferences. Checks for
# settings.json rather than the directory itself, because a previous launch may
# have already created an empty config dir.
if [[ -d $HOME/.config/zed && ! -f $HOME/.config/zed-md-viewer/settings.json ]]; then
    mkdir -p "$HOME/.config/zed-md-viewer"
    cp -r "$HOME/.config/zed/." "$HOME/.config/zed-md-viewer/"
    echo "migrated config: $HOME/.config/zed -> $HOME/.config/zed-md-viewer"
fi

sed -e "s|@BIN@|$HOME/.local/bin/zed-md-viewer|g" \
    crates/zed/resources/zed-md-viewer.desktop \
    > "$HOME/.local/share/applications/zed-md-viewer.desktop"

update-desktop-database "$HOME/.local/share/applications" 2>/dev/null || true

# Offer it as the default app for Markdown MIME types.
for mime in text/markdown text/x-markdown; do
    xdg-mime default zed-md-viewer.desktop "$mime"
done

echo "installed:"
echo "  $HOME/.local/bin/zed-md-viewer"
echo "  $HOME/.local/share/applications/zed-md-viewer.desktop (MimeType: text/markdown, text/x-markdown)"
echo "default handler for text/markdown: $(xdg-mime query default text/markdown)"
