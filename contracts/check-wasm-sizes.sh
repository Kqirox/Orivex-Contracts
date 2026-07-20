#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# check-wasm-sizes.sh – Verify Stellar contract WASM artifacts stay within
# their configured size budgets.
#
# Default budget: 50,000 bytes (~49 KB), well under the 64 KB protocol limit.
# Override per contract by editing the BUDGETS associative array below.
#
# Exit status: 0 = all contracts pass, 1 = one or more contracts over budget.
# ---------------------------------------------------------------------------
set -euo pipefail

# ----- configurable budgets (bytes) ----------------------------------------
DEFAULT_BUDGET=50000

declare -A BUDGETS=(
  ["course-registry"]=50000
  ["badge-nft"]=50000
  ["reward-pool"]=50000
  ["stake-vault"]=50000
  ["governance"]=50000
  ["quest-engine"]=50000
)

# ----- helpers -------------------------------------------------------------
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

WASM_DIR="${1:-target/wasm32v1-none/release}"

if [ ! -d "$WASM_DIR" ]; then
  echo "ERROR: WASM directory not found: $WASM_DIR"
  echo "Run 'stellar contract build' first."
  exit 1
fi

shopt -s nullglob
WASM_FILES=("$WASM_DIR"/*.wasm)
shopt -u nullglob

if [ ${#WASM_FILES[@]} -eq 0 ]; then
  echo "ERROR: No .wasm files found in $WASM_DIR"
  exit 1
fi

# ----- main ----------------------------------------------------------------
EXIT_CODE=0
declare -A SIZES
declare -A STATUSES
declare -A BUDGETS_FINAL

# Collect sizes first
for wasm in "${WASM_FILES[@]}"; do
  filename=$(basename "$wasm" .wasm)
  size=$(stat -c%s "$wasm")
  budget=${BUDGETS[$filename]:-$DEFAULT_BUDGET}
  SIZES[$filename]=$size
  BUDGETS_FINAL[$filename]=$budget

  if [ "$size" -lt "$budget" ]; then
    STATUSES[$filename]="PASS"
  else
    STATUSES[$filename]="FAIL"
    EXIT_CODE=1
  fi
done

# Print markdown report (for CI / PR comment)
echo "## 📊 WASM Size Report"
echo ""
echo "| Contract | Size (bytes) | Budget (bytes) | % Used | Status |"
echo "|----------|-------------|----------------|--------|--------|"

for wasm in "${WASM_FILES[@]}"; do
  filename=$(basename "$wasm" .wasm)
  size=${SIZES[$filename]}
  budget=${BUDGETS_FINAL[$filename]}
  pct=$(( size * 100 / budget ))
  status=${STATUSES[$filename]}

  if [ "$status" = "PASS" ]; then
    emoji="✅"
    color="$GREEN"
  else
    emoji="❌"
    color="$RED"
  fi

  printf "| \`%s\` | %'d | %'d | %d%% | %s %s |\n" \
    "$filename" "$size" "$budget" "$pct" "$emoji" "$status"

  # Also print colored terminal line (only when stdout is a terminal)
  if [ -t 1 ]; then
    echo -e "${color}$filename: $size / $budget bytes (${pct}%) — ${status}${NC}"
  fi
done

echo ""

if [ $EXIT_CODE -eq 0 ]; then
  echo "✅ **All contracts within size budget.**"
else
  echo "❌ **One or more contracts exceed the size budget!**"
fi

exit $EXIT_CODE
