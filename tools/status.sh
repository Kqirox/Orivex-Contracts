#!/usr/bin/env bash
#
# status.sh — Health report for deployed Orivex contracts.
#
# Usage:
#   ./tools/status.sh <network>
#
# Queries each contract and prints a health summary.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
DEPLOYMENTS_DIR="$REPO_ROOT/deployments"

# ── Args ─────────────────────────────────────────────────────────────────────

NETWORK="${1:-}"
if [[ -z "$NETWORK" ]]; then
  echo "Usage: $0 <network>"
  echo "  network: standalone | testnet | futurenet"
  exit 1
fi

DEPLOYMENT_FILE="$DEPLOYMENTS_DIR/${NETWORK}.json"

if [[ ! -f "$DEPLOYMENT_FILE" ]]; then
  echo "Error: No deployment found for '$NETWORK' at $DEPLOYMENT_FILE"
  exit 1
fi

# ── Helpers ──────────────────────────────────────────────────────────────────

STELLAR="stellar"

invoke_read() {
  local contract_id="$1"; shift
  $STELLAR contract invoke \
    --id "$contract_id" \
    --network "$NETWORK" \
    --rpc-url "${STELLAR_RPC_URL:-}" \
    --network-passphrase "${STELLAR_NETWORK_PASSPHRASE:-}" \
    -- "$@"
}

query_contract() {
  local name="$1"
  local contract_id="$2"
  shift 2

  local result
  if result=$(invoke_read "$contract_id" "$@" 2>/dev/null); then
    printf "  ✓ %-20s %s\n" "$name" "$result"
    return 0
  else
    printf "  ✗ %-20s UNREACHABLE\n" "$name"
    return 1
  fi
}

# ── Load deployment ──────────────────────────────────────────────────────────

declare -A CONTRACT_IDS

while IFS='=' read -r key value; do
  key="${key// /}"
  value="${value// /}"
  if [[ -n "$key" && "$key" != "{" && "$key" != "}" && "$key" != *":"* && "$key" != *"network"* && "$key" != *"admin"* && "$key" != *"token"* ]]; then
    CONTRACT_IDS["$key"]="${value//\"/}"
  fi
done < <(sed 's/^[[:space:]]*//' "$DEPLOYMENT_FILE" | grep -v '^\[' | tr -d ',' | sed 's/"//g')

# ── Report ───────────────────────────────────────────────────────────────────

echo "╔══════════════════════════════════════════════════════════╗"
echo "║        Orivex Contracts — Health Report                 ║"
echo "║  Network: $NETWORK"
echo "╚══════════════════════════════════════════════════════════╝"
echo ""

ERRORS=0

echo "Contract Addresses:"
for name in badge-nft reward-pool stake-vault course-registry quest-engine governance; do
  cid="${CONTRACT_IDS[$name]:-}"
  if [[ -z "$cid" ]]; then
    printf "  ● %-20s NOT DEPLOYED\n" "$name"
    ERRORS=$((ERRORS + 1))
  else
    printf "  · %-20s %s\n" "$name" "$cid"
  fi
done

echo ""
echo "Read Queries:"

# badge-nft: get_badge_count for admin (should be 0 or work without error)
query_contract "badge-nft" "${CONTRACT_IDS[badge_nft]:-missing}" --help >/dev/null 2>&1 && \
  printf "  ✓ %-20s deployed\n" "badge-nft" || { printf "  ✗ %-20s UNREACHABLE\n" "badge-nft"; ERRORS=$((ERRORS + 1)); }

# reward-pool: pool_balance
query_contract "reward-pool" "${CONTRACT_IDS[reward_pool]:-missing}" pool_balance || ERRORS=$((ERRORS + 1))

# stake-vault: (no simple read without user, check admin)
query_contract "stake-vault" "${CONTRACT_IDS[stake_vault]:-missing}" --help >/dev/null 2>&1 && \
  printf "  ✓ %-20s deployed\n" "stake-vault" || { printf "  ✗ %-20s UNREACHABLE\n" "stake-vault"; ERRORS=$((ERRORS + 1)); }

# course-registry: course_count
query_contract "course-registry" "${CONTRACT_IDS[course_registry]:-missing}" course_count || ERRORS=$((ERRORS + 1))

# quest-engine: (no zero-arg read, just check deploy)
query_contract "quest-engine" "${CONTRACT_IDS[quest_engine]:-missing}" --help >/dev/null 2>&1 && \
  printf "  ✓ %-20s deployed\n" "quest-engine" || { printf "  ✗ %-20s UNREACHABLE\n" "quest-engine"; ERRORS=$((ERRORS + 1)); }

# governance: (no zero-arg read, just check deploy)
query_contract "governance" "${CONTRACT_IDS[governance]:-missing}" --help >/dev/null 2>&1 && \
  printf "  ✓ %-20s deployed\n" "governance" || { printf "  ✗ %-20s UNREACHABLE\n" "governance"; ERRORS=$((ERRORS + 1)); }

echo ""
if [[ $ERRORS -eq 0 ]]; then
  echo "Result: ALL HEALTHY"
else
  echo "Result: $ERRORS ISSUE(S) FOUND"
fi
