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

systemctl --user show-environment &>/dev/null \
    || die "systemd user session is not running. Log in as a desktop user and try again."
ok "systemd user session is active"

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

# ── Systemd service ───────────────────────────────────────────────────────────

step "Installing systemd user service"
mkdir -p "$HOME/.config/systemd/user"

cat > "$HOME/.config/systemd/user/meetingtime.service" <<EOF
[Unit]
Description=Meeting reminder daemon

[Service]
ExecStart=%h/.cargo/bin/meetingtime run
Restart=on-failure
Environment=MEETINGTIME_CLIENT_ID=${CLIENT_ID}
Environment=MEETINGTIME_CLIENT_SECRET=${CLIENT_SECRET}

[Install]
WantedBy=default.target
EOF
ok "Service file written → ~/.config/systemd/user/meetingtime.service"

systemctl --user daemon-reload
systemctl --user stop meetingtime 2>/dev/null || true
systemctl --user enable --now meetingtime
ok "Service enabled and started"

# ── OAuth ─────────────────────────────────────────────────────────────────────

step "Google OAuth authorisation"
printf "  Your browser will open. Authorise meetingtime, then paste the\n"
printf "  redirect URL (or just the code= value) back here.\n\n"
MEETINGTIME_CLIENT_ID="$CLIENT_ID" MEETINGTIME_CLIENT_SECRET="$CLIENT_SECRET" \
    "$HOME/.cargo/bin/meetingtime" auth

# ── .zshrc ────────────────────────────────────────────────────────────────────

step "Adding credentials to ~/.zshrc"
ZSHRC="$HOME/.zshrc"
if grep -q 'MEETINGTIME_CLIENT_ID' "$ZSHRC" 2>/dev/null; then
    ok "~/.zshrc already contains MEETINGTIME_CLIENT_ID — skipping"
else
    touch "$ZSHRC"
    cat >> "$ZSHRC" <<EOF

# meetingtime credentials (for manual CLI use; daemon reads these from the service file)
export MEETINGTIME_CLIENT_ID="${CLIENT_ID}"
export MEETINGTIME_CLIENT_SECRET="${CLIENT_SECRET}"
EOF
    ok "Credentials appended → ~/.zshrc"
fi

# ── Verify ────────────────────────────────────────────────────────────────────

step "Verifying service"
sleep 2
if systemctl --user is-active --quiet meetingtime; then
    ok "meetingtime.service is active"
else
    printf "  ${RED}✗ Service is not active${RESET}\n"
fi
systemctl --user status meetingtime --no-pager || true

# ── Summary ───────────────────────────────────────────────────────────────────

printf "\n${BOLD}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}\n"
printf "${GREEN}✓${RESET} Binary      → ~/.cargo/bin/meetingtime\n"
printf "${GREEN}✓${RESET} Token       → ~/.local/share/meetingtime/token.json\n"
printf "${GREEN}✓${RESET} Daemon      → systemctl --user status meetingtime\n"
printf "${GREEN}✓${RESET} Credentials → ~/.zshrc\n"
printf "${BOLD}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}\n"
printf "\nReload your shell:  source ~/.zshrc\n"
