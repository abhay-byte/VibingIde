#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
LAUNCHER="${SCRIPT_DIR}/launch-vibingide.sh"

DESKTOP_DIR="${XDG_DESKTOP_DIR:-${HOME}/Desktop}"
APPLICATIONS_DIR="${XDG_DATA_HOME:-${HOME}/.local/share}/applications"
AUTOSTART_DIR="${XDG_CONFIG_HOME:-${HOME}/.config}/autostart"

DESKTOP_ENTRY="${DESKTOP_DIR}/VibingIDE.desktop"
APPLICATION_ENTRY="${APPLICATIONS_DIR}/VibingIDE.desktop"
AUTOSTART_ENTRY="${AUTOSTART_DIR}/VibingIDE.desktop"

mkdir -p "${DESKTOP_DIR}" "${APPLICATIONS_DIR}" "${AUTOSTART_DIR}"
chmod +x "${LAUNCHER}"

if [[ -x "${REPO_ROOT}/target/release/vibingide" ]]; then
  ICON_PATH="${REPO_ROOT}/target/release/vibingide"
elif [[ -x "${REPO_ROOT}/target/debug/vibingide" ]]; then
  ICON_PATH="${REPO_ROOT}/target/debug/vibingide"
else
  ICON_PATH="utilities-terminal"
fi

create_entry() {
  local target_path="$1"
  local terminal_flag="$2"

  cat > "${target_path}" <<EOF
[Desktop Entry]
Version=1.0
Type=Application
Name=VibingIDE
Comment=Launch VibingIDE
Exec=/usr/bin/env bash "${LAUNCHER}"
Path=${REPO_ROOT}
Terminal=${terminal_flag}
Icon=${ICON_PATH}
Categories=Development;
StartupNotify=true
EOF

  chmod +x "${target_path}"
}

create_entry "${DESKTOP_ENTRY}" "true"
create_entry "${APPLICATION_ENTRY}" "true"
create_entry "${AUTOSTART_ENTRY}" "false"

printf 'Updated Linux desktop entries:\n'
printf ' - %s\n' "${DESKTOP_ENTRY}"
printf ' - %s\n' "${APPLICATION_ENTRY}"
printf ' - %s\n' "${AUTOSTART_ENTRY}"
