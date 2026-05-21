#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_ROOT"

# ── Configuration ──────────────────────────────────────────
PROFILE="${1:-release}"
TARGET="wasm32-unknown-unknown"
PACKAGE="wave-milestone"

# ── Build ──────────────────────────────────────────────────
echo "==> Building $PACKAGE ($PROFILE) for $TARGET..."

if [ "$PROFILE" = "release" ]; then
    cargo build --release --target "$TARGET" --package "$PACKAGE"
else
    cargo build --target "$TARGET" --package "$PACKAGE"
fi

echo "==> Build complete."

# ── Optimize WASM (release only) ──────────────────────────
if [ "$PROFILE" = "release" ]; then
    WASM_FILE="target/$TARGET/release/${PACKAGE//-/_}.wasm"

    if command -v wasm-opt &>/dev/null; then
        echo "==> Optimizing WASM with wasm-opt..."
        wasm-opt \
            -Os \
            --strip-debug \
            --enable-bulk-memory \
            "$WASM_FILE" \
            -o "${WASM_FILE%.wasm}_optimized.wasm"
        echo "==> Optimized: ${WASM_FILE%.wasm}_optimized.wasm"

        # Show size comparison
        ORIG_SIZE=$(stat -c%s "$WASM_FILE")
        OPT_SIZE=$(stat -c%s "${WASM_FILE%.wasm}_optimized.wasm")
        echo "==> Size: $ORIG_SIZE bytes -> $OPT_SIZE bytes ($(( (ORIG_SIZE - OPT_SIZE) * 100 / ORIG_SIZE ))% reduction)"
    else
        echo "==> wasm-opt not found; skipping optimization."
        echo "    Install binaryen: apt install binaryen / brew install binaryen"
    fi

    # Compute SHA-256
    sha256sum "$WASM_FILE" > "${WASM_FILE}.sha256"
    echo "==> SHA-256: $(cat "${WASM_FILE}.sha256")"
fi

echo "==> Done."
