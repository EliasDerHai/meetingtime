#!/usr/bin/env bash
set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[1;34m'
BOLD='\033[1m'
RESET='\033[0m'

step() { printf "\n${BLUE}==> %s${RESET}\n" "$*"; }
ok()   { printf "  ${GREEN}✓${RESET} %s\n" "$*"; }
die()  { printf "\n${RED}ERROR: %s${RESET}\n" "$*" >&2; exit 1; }

PLIST_LABEL="com.elias.meetingtime"
PLIST_PATH="$HOME/Library/LaunchAgents/${PLIST_LABEL}.plist"

# ── Preflight ────────────────────────────────────────────────────────────────

step "Checking prerequisites"

[[ -f Cargo.toml ]] \
    || die "Run this script from the meetingtime repo root (Cargo.toml not found in $PWD)"

if ! command -v cargo &>/dev/null; then
    printf "  ${BOLD}Rust/cargo not found.${RESET} Install via rustup? [Y/n] "
    read -r answer
    if [[ "${answer,,}" == "n" ]]; then
        die "Rust is required. Install from https://rustup.rs and re-run this script."
    fi
    step "Installing Rust via rustup"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    # shellcheck source=/dev/null
    source "$HOME/.cargo/env"
fi
ok "cargo $(cargo --version)"

# ── Credentials ──────────────────────────────────────────────────────────────

step "Google OAuth credentials"
printf "  Create a Desktop app client at:\n"
printf "  https://console.cloud.google.com/apis/credentials\n\n"
read -rp  "  Client ID:     " CLIENT_ID
read -rp  "  Client Secret: " CLIENT_SECRET
[[ -n "$CLIENT_ID" ]]     || die "Client ID cannot be empty"
[[ -n "$CLIENT_SECRET" ]] || die "Client Secret cannot be empty"

# ── Build ─────────────────────────────────────────────────────────────────────

step "Building release binary"
cargo build --release
ok "Build complete"

# ── Install binary ────────────────────────────────────────────────────────────

step "Installing binary"
mkdir -p "$HOME/.cargo/bin"
cp target/release/meetingtime "$HOME/.cargo/bin/meetingtime"
ok "Installed → $HOME/.cargo/bin/meetingtime"

# ── launchd plist ─────────────────────────────────────────────────────────────

step "Installing launchd agent"
mkdir -p "$HOME/Library/LaunchAgents"

cat > "$PLIST_PATH" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>Label</key><string>${PLIST_LABEL}</string>
  <key>ProgramArguments</key><array>
    <string>${HOME}/.cargo/bin/meetingtime</string>
    <string>run</string>
  </array>
  <key>EnvironmentVariables</key><dict>
    <key>MEETINGTIME_CLIENT_ID</key><string>${CLIENT_ID}</string>
    <key>MEETINGTIME_CLIENT_SECRET</key><string>${CLIENT_SECRET}</string>
  </dict>
  <key>StandardOutPath</key><string>/tmp/meetingtime.log</string>
  <key>StandardErrorPath</key><string>/tmp/meetingtime.err</string>
  <key>RunAtLoad</key><true/>
  <key>KeepAlive</key><true/>
</dict></plist>
EOF

# Restrict permissions — plist contains secrets
chmod 600 "$PLIST_PATH"
ok "Plist written → $PLIST_PATH"

# ── OAuth ─────────────────────────────────────────────────────────────────────

step "Google OAuth authorisation"
printf "  Your browser will open. Authorise meetingtime, then paste the\n"
printf "  redirect URL (or just the code= value) back here.\n\n"
MEETINGTIME_CLIENT_ID="$CLIENT_ID" MEETINGTIME_CLIENT_SECRET="$CLIENT_SECRET" \
    "$HOME/.cargo/bin/meetingtime" auth

# ── Load service ──────────────────────────────────────────────────────────────

step "Loading launchd agent"
# Unload first if already running so the new plist takes effect
launchctl unload "$PLIST_PATH" 2>/dev/null || true
launchctl load "$PLIST_PATH"
ok "Agent loaded"

# ── .zshrc ────────────────────────────────────────────────────────────────────

step "Adding credentials to ~/.zshrc"
ZSHRC="$HOME/.zshrc"
if grep -q 'MEETINGTIME_CLIENT_ID' "$ZSHRC" 2>/dev/null; then
    ok "~/.zshrc already contains MEETINGTIME_CLIENT_ID — skipping"
else
    touch "$ZSHRC"
    cat >> "$ZSHRC" <<EOF

# meetingtime credentials (for manual CLI use; daemon reads these from the launchd plist)
export MEETINGTIME_CLIENT_ID="${CLIENT_ID}"
export MEETINGTIME_CLIENT_SECRET="${CLIENT_SECRET}"
EOF
    ok "Credentials appended → ~/.zshrc"
fi

# ── Verify ────────────────────────────────────────────────────────────────────

step "Verifying launchd agent"
sleep 2
LAUNCHCTL_LINE=$(launchctl list | grep "$PLIST_LABEL" || true)
if [[ -z "$LAUNCHCTL_LINE" ]]; then
    printf "  ${RED}✗ Agent not found in launchctl list${RESET}\n"
    printf "  Check logs: cat /tmp/meetingtime.err\n"
else
    PID=$(echo "$LAUNCHCTL_LINE" | awk '{print $1}')
    STATUS=$(echo "$LAUNCHCTL_LINE" | awk '{print $2}')
    if [[ "$PID" != "-" ]]; then
        ok "Agent running (PID $PID)"
    else
        printf "  ${RED}✗ Agent not running (last exit status: $STATUS)${RESET}\n"
        printf "  Check logs: cat /tmp/meetingtime.err\n"
    fi
fi

# ── Summary ───────────────────────────────────────────────────────────────────

printf "\n${BOLD}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}\n"
printf "${GREEN}✓${RESET} Binary      → ~/.cargo/bin/meetingtime\n"
printf "${GREEN}✓${RESET} Token       → ~/Library/Application Support/meetingtime/token.json\n"
printf "${GREEN}✓${RESET} Daemon      → launchctl list | grep ${PLIST_LABEL}\n"
printf "${GREEN}✓${RESET} Credentials → ~/.zshrc\n"
printf "${BOLD}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}\n"
printf "\nReload your shell:  source ~/.zshrc\n"
