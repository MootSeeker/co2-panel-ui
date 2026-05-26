#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
INSTALL_PREFIX="${CO2_PANEL_PREFIX:-/opt/co2-panel}"
AUTOSTART_DIR="${HOME}/.config/autostart"

install_system_dependencies() {
  sudo apt-get update
  sudo apt-get install -y \
    build-essential \
    curl \
    libasound2-dev \
    libegl1-mesa-dev \
    libfontconfig1-dev \
    libgl1-mesa-dev \
    libwayland-dev \
    libx11-dev \
    libxcb1-dev \
    libxcursor-dev \
    libxi-dev \
    libxinerama-dev \
    libxkbcommon-dev \
    libxrandr-dev \
    pkg-config
}

install_rust_if_missing() {
  if command -v cargo >/dev/null 2>&1; then
    return
  fi

  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
  # shellcheck disable=SC1091
  source "${HOME}/.cargo/env"
}

build_project() {
  cargo build --release
}

install_files() {
  sudo install -d \
    "${INSTALL_PREFIX}/bin" \
    "${INSTALL_PREFIX}/include" \
    "${INSTALL_PREFIX}/lib" \
    "${INSTALL_PREFIX}/docs" \
    "${INSTALL_PREFIX}/examples"

  sudo install -m 755 "${ROOT_DIR}/target/release/co2_panel_ui" "${INSTALL_PREFIX}/bin/co2_panel_ui"
  sudo install -m 644 "${ROOT_DIR}/target/release/libco2_panel_c_api.a" "${INSTALL_PREFIX}/lib/libco2_panel_c_api.a"
  sudo install -m 644 "${ROOT_DIR}/include/co2_panel.h" "${INSTALL_PREFIX}/include/co2_panel.h"
  sudo cp -R "${ROOT_DIR}/docs/"* "${INSTALL_PREFIX}/docs/"
  sudo cp -R "${ROOT_DIR}/examples/"* "${INSTALL_PREFIX}/examples/"
}

install_autostart() {
  mkdir -p "${AUTOSTART_DIR}"
  cat > "${AUTOSTART_DIR}/co2-panel.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=CO2 Panel
Comment=Startet das CO2 Panel im Vollbild
Exec=${INSTALL_PREFIX}/bin/co2_panel_ui
Terminal=false
X-GNOME-Autostart-enabled=true
EOF
}

main() {
  install_system_dependencies
  install_rust_if_missing
  build_project
  install_files
  install_autostart

  echo "CO2 Panel wurde installiert."
  echo "UI: ${INSTALL_PREFIX}/bin/co2_panel_ui"
  echo "Header: ${INSTALL_PREFIX}/include/co2_panel.h"
  echo "Library: ${INSTALL_PREFIX}/lib/libco2_panel_c_api.a"
}

main "$@"
