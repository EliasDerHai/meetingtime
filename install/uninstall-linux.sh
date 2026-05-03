#!/usr/bin/env bash
set -euo pipefail

GREEN='\033[0;32m'
BLUE='\033[1;34m'
BOLD='\033[1m'
RESET='\033[0m'

step() { printf '\n%s==> %s%s\n' "${BLUE}" "$*" "${RESET}"; }
ok()   { printf '  %s✓%s %s\n' "${GREEN}" "${RESET}" "$*"; }

# ── Stop daemon ───────────────────────────────────────────────────────────────

step "Stopping systemd service"
systemctl --user stop meetingtime 2>/dev/null || true
systemctl --user disable meetingtime 2>/dev/null || true
ok "Service stopped and disabled"

# ── Remove service file ───────────────────────────────────────────────────────

step "Removing systemd service file"
rm -f "$HOME/.config/systemd/user/meetingtime.service"
systemctl --user daemon-reload
ok "Removed → ~/.config/systemd/user/meetingtime.service"

# ── Remove binary ─────────────────────────────────────────────────────────────

step "Removing binary"
rm -f "$HOME/.cargo/bin/meetingtime"
ok "Removed → ~/.cargo/bin/meetingtime"

# ── Remove data & config ──────────────────────────────────────────────────────

step "Removing token and config"
rm -rf "$HOME/.local/share/meetingtime"
ok "Removed → ~/.local/share/meetingtime"
rm -rf "$HOME/.config/meetingtime"
ok "Removed → ~/.config/meetingtime"

# ── ~/.zshrc reminder ─────────────────────────────────────────────────────────

printf '\n%sTo finish, remove these lines from ~/.zshrc:%s\n' "${BOLD}" "${RESET}"
printf '  # meetingtime credentials (for manual CLI use; daemon reads these from the service file)\n'
printf '  export MEETINGTIME_CLIENT_ID="..."\n'
printf '  export MEETINGTIME_CLIENT_SECRET="..."\n'

# ── Summary ───────────────────────────────────────────────────────────────────

printf '\n%s━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━%s\n' "${BOLD}" "${RESET}"
printf '%s✓%s Binary      → ~/.cargo/bin/meetingtime\n'                "${GREEN}" "${RESET}"
printf '%s✓%s Service     → ~/.config/systemd/user/meetingtime.service\n' "${GREEN}" "${RESET}"
printf '%s✓%s Token       → ~/.local/share/meetingtime/\n'             "${GREEN}" "${RESET}"
printf '%s✓%s Config      → ~/.config/meetingtime/\n'                  "${GREEN}" "${RESET}"
printf '%s━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━%s\n'    "${BOLD}"  "${RESET}"
