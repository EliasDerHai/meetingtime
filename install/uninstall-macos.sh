#!/usr/bin/env bash
set -euo pipefail

GREEN='\033[0;32m'
BLUE='\033[1;34m'
BOLD='\033[1m'
RESET='\033[0m'

step() { printf '\n%s==> %s%s\n' "${BLUE}" "$*" "${RESET}"; }
ok()   { printf '  %s✓%s %s\n' "${GREEN}" "${RESET}" "$*"; }

PLIST_LABEL="com.elias.meetingtime"
PLIST_PATH="$HOME/Library/LaunchAgents/${PLIST_LABEL}.plist"

# ── Stop daemon ───────────────────────────────────────────────────────────────

step "Stopping launchd agent"
launchctl unload "$PLIST_PATH" 2>/dev/null || true
ok "Agent unloaded"

# ── Remove plist ──────────────────────────────────────────────────────────────

step "Removing launchd plist"
rm -f "$PLIST_PATH"
ok "Removed → $PLIST_PATH"

# ── Remove binary ─────────────────────────────────────────────────────────────

step "Removing binary"
rm -f "$HOME/.cargo/bin/meetingtime"
ok "Removed → ~/.cargo/bin/meetingtime"

# ── Remove data & config ──────────────────────────────────────────────────────

step "Removing token and config"
rm -rf "$HOME/Library/Application Support/meetingtime"
ok "Removed → ~/Library/Application Support/meetingtime"
rm -rf "$HOME/.config/meetingtime"
ok "Removed → ~/.config/meetingtime"

# ── ~/.zshrc reminder ─────────────────────────────────────────────────────────

printf '\n%sTo finish, remove these lines from ~/.zshrc:%s\n' "${BOLD}" "${RESET}"
printf '  # meetingtime credentials (for manual CLI use; daemon reads these from the launchd plist)\n'
printf '  export MEETINGTIME_CLIENT_ID="..."\n'
printf '  export MEETINGTIME_CLIENT_SECRET="..."\n'

# ── Summary ───────────────────────────────────────────────────────────────────

printf '\n%s━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━%s\n' "${BOLD}" "${RESET}"
printf '%s✓%s Binary      → ~/.cargo/bin/meetingtime\n'                   "${GREEN}" "${RESET}"
printf '%s✓%s Plist       → ~/Library/LaunchAgents/%s.plist\n'            "${GREEN}" "${RESET}" "${PLIST_LABEL}"
printf '%s✓%s Token       → ~/Library/Application Support/meetingtime/\n' "${GREEN}" "${RESET}"
printf '%s✓%s Config      → ~/.config/meetingtime/\n'                     "${GREEN}" "${RESET}"
printf '%s━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━%s\n'       "${BOLD}"  "${RESET}"
