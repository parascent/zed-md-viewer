#!/usr/bin/env bash
# Builds and starts the Markdown editor for testing.
#
#   ./dev.sh                 # debug build (fast) + test fixture
#   ./dev.sh notes.md        # debug build + your file(s)
#   ./dev.sh --release       # release build + test fixture
#
# The debug build reads assets/keymaps and assets/settings from the checkout at
# runtime, so keymap/settings edits apply on the next launch without a rebuild.
set -euo pipefail

cd "$(dirname "$0")"

profile=debug
files=()
while [[ $# -gt 0 ]]; do
    case "$1" in
        --release)
            profile=release
            ;;
        -h|--help)
            sed -n '2,8p' "$0" | sed 's/^# \{0,1\}//'
            exit 0
            ;;
        *)
            files+=("$1")
            ;;
    esac
    shift
done

if [[ ${#files[@]} -eq 0 ]]; then
    fixture="$HOME/.cache/zed-md-viewer-test/test.md"
    mkdir -p "$(dirname "$fixture")"
    if [[ ! -f $fixture ]]; then
        cat > "$fixture" <<'EOF'
# zed-md-viewer test file

Some text with **bold**, *italic*, and `inline code`.

- bullet one
- bullet two

| Name | Qty | Notes |
| ---- | ---: | ----- |
| apple | 1 | fresh |
| pear | 22 | |

```rust
fn main() {
    println!("hello");
}
```

Put the cursor in the table above and press ctrl-alt-t.
EOF
    fi
    files=("$fixture")
fi

echo "building ($profile)..."
if [[ $profile == release ]]; then
    cargo build --release -p zed
    bin=target/release/zed-md-viewer
else
    cargo build -p zed
    bin=target/debug/zed-md-viewer
fi

echo "starting $bin ${files[*]}"
exec "$bin" "${files[@]}"
