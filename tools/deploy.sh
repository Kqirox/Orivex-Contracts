#!/usr/bin/env bash
#
# deploy.sh — Deploy all Orivex contracts to a Stellar network.
#
# Usage:
#   ./tools/deploy.sh <network>
#
# Networks: standalone | testnet | futurenet
#
# Idempotent: re-running on a fully-deployed state skips already-deployed
# contracts and only performs missing wiring steps.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
CONTRACTS_DIR="$REPO_ROOT/contracts"
DEPLOYMENTS_DIR="$REPO_ROOT/deployments"
WASM_DIR="$CONTRACTS_DIR/target/wasm32-unknown-unknown/release"

# ── Args ─────────────────────────────────────────────────────────────────────

NETWORK="${1:-}"
if [[ -z "$NETWORK" ]]; then
  echo "Usage: $0 <network>"
  echo "  network: standalone | testnet | futurenet"
  exit 1
fi

case "$NETWORK" in
  standalone|testnet|futurenet) ;;
  *) echo "Error: unknown network '$NETWORK'. Use standalone, testnet, or futurenet."; exit 1 ;;
esac

DEPLOYMENT_FILE="$DEPLOYMENTS_DIR/${NETWORK}.json"

# ── Helpers ──────────────────────────────────────────────────────────────────

log()  { echo "▸ $*"; }
ok()   { echo "  ✓ $*"; }
skip() { echo "  ● $* (already deployed)"; }
fail() { echo "  ✗ $*"; exit 1; }

# Source account for deployments (reads STELLAR_SECRET_KEY or prompts)
if [[ -z "${STELLAR_SECRET_KEY:-}" ]]; then
  echo "Error: STELLAR_SECRET_KEY environment variable is required."
  exit 1
fi

ADMIN="$STELLAR_SECRET_KEY"

# Token address for contracts that need it (reward-pool, quest-engine, stake-vault)
TOKEN="${STELLAR_TOKEN_ADDRESS:-}"
if [[ -z "$TOKEN" ]]; then
  echo "Error: STELLAR_TOKEN_ADDRESS environment variable is required (USDC token address)."
  exit 1
fi

# stellar CLI wrapper
STELLAR="stellar"

# Deploy a contract and return its address.
# Args: $1 = contract name (for wasm lookup)
deploy_contract() {
  local name="$1"
  local wasm="$WASM_DIR/${name}.wasm"

  if [[ ! -f "$wasm" ]]; then
    fail "WASM not found: $wasm — run 'stellar contract build' first."
  fi

  local addr
  addr=$($STELLAR contract deploy \
    --wasm "$wasm" \
    --source-account "$ADMIN" \
    --network "$NETWORK" \
    --rpc-url "${STELLAR_RPC_URL:-}" \
    --network-passphrase "${STELLAR_NETWORK_PASSPHRASE:-}" \
    2>/dev/null)

  echo "$addr"
}

# Invoke a contract function.
# Args: $1 = contract_id, ... rest = invoke args
invoke() {
  local contract_id="$1"; shift
  $STELLAR contract invoke \
    --id "$contract_id" \
    --source-account "$ADMIN" \
    --network "$NETWORK" \
    --rpc-url "${STELLAR_RPC_URL:-}" \
    --network-passphrase "${STELLAR_NETWORK_PASSPHRASE:-}" \
    -- "$@"
}

# Read-only invoke (no source account needed).
invoke_read() {
  local contract_id="$1"; shift
  $STELLAR contract invoke \
    --id "$contract_id" \
    --network "$NETWORK" \
    --rpc-url "${STELLAR_RPC_URL:-}" \
    --network-passphrase "${STELLAR_NETWORK_PASSPHRASE:-}" \
    -- "$@"
}

# Check if a contract exists by attempting a read.
contract_exists() {
  local contract_id="$1"
  invoke_read "$contract_id" --help >/dev/null 2>&1 || \
    $STELLAR contract inspect --id "$contract_id" --network "$NETWORK" \
      --rpc-url "${STELLAR_RPC_URL:-}" \
      --network-passphrase "${STELLAR_NETWORK_PASSPHRASE:-}" >/dev/null 2>&1
}

# ── Load existing deployment ─────────────────────────────────────────────────

declare -A CONTRACT_IDS

load_deployment() {
  if [[ -f "$DEPLOYMENT_FILE" ]]; then
    log "Loading existing deployment from $DEPLOYMENT_FILE"
    while IFS='=' read -r key value; do
      key="${key// /}"
      value="${value// /}"
      if [[ -n "$key" && "$key" != "{" && "$key" != "}" && "$key" != *":"* ]]; then
        CONTRACT_IDS["$key"]="${value//\"/}"
      fi
    done < <(sed 's/^[[:space:]]*//' "$DEPLOYMENT_FILE" | grep -v '^\[' | tr -d ',' | sed 's/"//g')
  fi
}

save_deployment() {
  mkdir -p "$DEPLOYMENTS_DIR"
  cat > "$DEPLOYMENT_FILE" <<EOF
{
  "network": "$NETWORK",
  "admin": "$(echo "$ADMIN" | sed 's/\(.\{4\}\).*/\1.../'  )",
  "token": "$TOKEN",
  "badge_nft": "${CONTRACT_IDS[badge_nft]:-}",
  "reward_pool": "${CONTRACT_IDS[reward_pool]:-}",
  "stake_vault": "${CONTRACT_IDS[stake_vault]:-}",
  "course_registry": "${CONTRACT_IDS[course_registry]:-}",
  "quest_engine": "${CONTRACT_IDS[quest_engine]:-}",
  "governance": "${CONTRACT_IDS[governance]:-}"
}
EOF
  ok "Deployment config saved to $DEPLOYMENT_FILE"
}

# ── Build ────────────────────────────────────────────────────────────────────

log "Building all contracts..."
(cd "$CONTRACTS_DIR" && $STELLAR contract build)
ok "Build complete."

# ── Deploy & Initialize ─────────────────────────────────────────────────────

log "Deploying contracts to $NETWORK..."

# 1. badge-nft
if [[ -n "${CONTRACT_IDS[badge_nft]:-}" ]]; then
  skip "badge-nft (${CONTRACT_IDS[badge_nft]})"
else
  log "Deploying badge-nft..."
  CONTRACT_IDS[badge_nft]=$(deploy_contract "badge-nft")
  ok "badge-nft deployed at ${CONTRACT_IDS[badge_nft]}"
  invoke "${CONTRACT_IDS[badge_nft]}" initialize --admin "$ADMIN"
  ok "badge-nft initialized"
fi

# 2. reward-pool
if [[ -n "${CONTRACT_IDS[reward_pool]:-}" ]]; then
  skip "reward-pool (${CONTRACT_IDS[reward_pool]})"
else
  log "Deploying reward-pool..."
  CONTRACT_IDS[reward_pool]=$(deploy_contract "reward-pool")
  ok "reward-pool deployed at ${CONTRACT_IDS[reward_pool]}"
  invoke "${CONTRACT_IDS[reward_pool]}" initialize --admin "$ADMIN" --token "$TOKEN"
  ok "reward-pool initialized"
fi

# 3. stake-vault
if [[ -n "${CONTRACT_IDS[stake_vault]:-}" ]]; then
  skip "stake-vault (${CONTRACT_IDS[stake_vault]})"
else
  log "Deploying stake-vault..."
  CONTRACT_IDS[stake_vault]=$(deploy_contract "stake-vault")
  ok "stake-vault deployed at ${CONTRACT_IDS[stake_vault]}"
  invoke "${CONTRACT_IDS[stake_vault]}" initialize --admin "$ADMIN" --token "$TOKEN"
  ok "stake-vault initialized"
fi

# 4. course-registry (needs badge-nft + reward-pool addresses)
if [[ -n "${CONTRACT_IDS[course_registry]:-}" ]]; then
  skip "course-registry (${CONTRACT_IDS[course_registry]})"
else
  log "Deploying course-registry..."
  CONTRACT_IDS[course_registry]=$(deploy_contract "course-registry")
  ok "course-registry deployed at ${CONTRACT_IDS[course_registry]}"
  invoke "${CONTRACT_IDS[course_registry]}" initialize --admin "$ADMIN"
  ok "course-registry initialized"
fi

# 5. quest-engine (needs reward-pool + stake-vault addresses)
if [[ -n "${CONTRACT_IDS[quest_engine]:-}" ]]; then
  skip "quest-engine (${CONTRACT_IDS[quest_engine]})"
else
  log "Deploying quest-engine..."
  CONTRACT_IDS[quest_engine]=$(deploy_contract "quest-engine")
  ok "quest-engine deployed at ${CONTRACT_IDS[quest_engine]}"
  invoke "${CONTRACT_IDS[quest_engine]}" initialize \
    --admin "$ADMIN" \
    --token "$TOKEN" \
    --reward_pool "${CONTRACT_IDS[reward_pool]}" \
    --stake_vault "${CONTRACT_IDS[stake_vault]}"
  ok "quest-engine initialized"
fi

# 6. governance (needs badge-nft address)
if [[ -n "${CONTRACT_IDS[governance]:-}" ]]; then
  skip "governance (${CONTRACT_IDS[governance]})"
else
  log "Deploying governance..."
  CONTRACT_IDS[governance]=$(deploy_contract "governance")
  ok "governance deployed at ${CONTRACT_IDS[governance]}"
  invoke "${CONTRACT_IDS[governance]}" initialize \
    --admin "$ADMIN" \
    --badge_contract_address "${CONTRACT_IDS[badge_nft]}"
  ok "governance initialized"
fi

# ── Wire cross-contract addresses ────────────────────────────────────────────

log "Wiring cross-contract addresses..."

# course-registry → badge-nft
invoke "${CONTRACT_IDS[course_registry]}" set_badge_nft_address \
  --admin "$ADMIN" \
  --badge_nft_address "${CONTRACT_IDS[badge_nft]}"
ok "course-registry.set_badge_nft_address"

# course-registry → reward-pool
invoke "${CONTRACT_IDS[course_registry]}" set_reward_pool_address \
  --admin "$ADMIN" \
  --reward_pool_address "${CONTRACT_IDS[reward_pool]}"
ok "course-registry.set_reward_pool_address"

# reward-pool → course-registry (approved spender)
invoke "${CONTRACT_IDS[reward_pool]}" add_approved_spender \
  --admin "$ADMIN" \
  --spender "${CONTRACT_IDS[course_registry]}"
ok "reward-pool.add_approved_spender(course-registry)"

# reward-pool → quest-engine (approved spender)
invoke "${CONTRACT_IDS[reward_pool]}" add_approved_spender \
  --admin "$ADMIN" \
  --spender "${CONTRACT_IDS[quest_engine]}"
ok "reward-pool.add_approved_spender(quest-engine)"

# ── Save ─────────────────────────────────────────────────────────────────────

save_deployment

echo ""
log "Deployment complete for $NETWORK"
echo ""
echo "Contract IDs:"
for name in badge-nft reward-pool stake-vault course-registry quest-engine governance; do
  printf "  %-20s %s\n" "$name" "${CONTRACT_IDS[$name]:-}"
done
echo ""
echo "Config saved to: $DEPLOYMENT_FILE"
