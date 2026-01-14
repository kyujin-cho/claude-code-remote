#!/usr/bin/env bash
# Build script for creating self-executable binary using PEX + scie-jump
#
# Prerequisites:
#   - Python 3.10+
#   - pip install pex
#
# Usage:
#   ./scripts/build_scie.sh [eager|lazy]
#
# Output:
#   dist/claude-code-telegram-hook  (self-contained executable)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Configuration
SCIE_MODE="${1:-eager}"  # eager (bundled Python) or lazy (fetch on first run)
OUTPUT_DIR="$PROJECT_ROOT/dist"
OUTPUT_NAME="claude-code-telegram-hook"

# Validate scie mode
if [[ "$SCIE_MODE" != "eager" && "$SCIE_MODE" != "lazy" ]]; then
    echo "Error: Invalid scie mode. Use 'eager' or 'lazy'"
    echo "  eager: Bundle Python runtime (~50MB, works offline)"
    echo "  lazy:  Fetch Python on first run (~5MB, requires internet)"
    exit 1
fi

echo "=== Building claude-code-telegram-hook SCIE (mode: $SCIE_MODE) ==="

# Ensure output directory exists
mkdir -p "$OUTPUT_DIR"

# Check for pex
if ! command -v pex &> /dev/null; then
    echo "Error: pex not found. Install with: pip install pex"
    exit 1
fi

echo "Building PEX with scie-jump..."

cd "$PROJECT_ROOT"

# Build the scie executable
# --scie: Create self-contained executable with Python runtime
# --scie-python-version: Target Python version for the bundled interpreter
# -c: Console script entry point
# -o: Output file path
pex . \
    --scie "$SCIE_MODE" \
    --scie-python-version 3.11 \
    -c claude-code-telegram-hook \
    -o "$OUTPUT_DIR/$OUTPUT_NAME"

# Make executable (should already be, but ensure)
chmod +x "$OUTPUT_DIR/$OUTPUT_NAME"

echo ""
echo "=== Build complete ==="
echo "Output: $OUTPUT_DIR/$OUTPUT_NAME"
echo ""
echo "File size: $(du -h "$OUTPUT_DIR/$OUTPUT_NAME" | cut -f1)"
echo ""
echo "Test with:"
echo "  echo '{}' | $OUTPUT_DIR/$OUTPUT_NAME"
echo ""
echo "Inspect scie manifest:"
echo "  SCIE=inspect $OUTPUT_DIR/$OUTPUT_NAME | jq ."
