#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="${RELEASE_VERSION:-dev}"
PACKAGE_NAME="co2-panel-${VERSION}"
DIST_DIR="${ROOT_DIR}/dist"
STAGING_DIR="${DIST_DIR}/${PACKAGE_NAME}"

rm -rf "${STAGING_DIR}"
mkdir -p "${STAGING_DIR}"

mkdir -p \
  "${STAGING_DIR}/crates" \
  "${STAGING_DIR}/deploy" \
  "${STAGING_DIR}/docs" \
  "${STAGING_DIR}/examples" \
  "${STAGING_DIR}/include" \
  "${STAGING_DIR}/scripts"

cp "${ROOT_DIR}/Cargo.toml" "${STAGING_DIR}/"
if [[ -f "${ROOT_DIR}/Cargo.lock" ]]; then
  cp "${ROOT_DIR}/Cargo.lock" "${STAGING_DIR}/"
fi
cp "${ROOT_DIR}/README.md" "${STAGING_DIR}/"
cp -R "${ROOT_DIR}/crates/"* "${STAGING_DIR}/crates/"
cp -R "${ROOT_DIR}/deploy/"* "${STAGING_DIR}/deploy/"
cp -R "${ROOT_DIR}/docs/"* "${STAGING_DIR}/docs/"
cp -R "${ROOT_DIR}/examples/"* "${STAGING_DIR}/examples/"
cp -R "${ROOT_DIR}/include/"* "${STAGING_DIR}/include/"
cp "${ROOT_DIR}/scripts/install-on-raspberry-pi.sh" "${STAGING_DIR}/install.sh"

chmod +x "${STAGING_DIR}/install.sh"

cat > "${STAGING_DIR}/PAKET-LESEN.md" <<EOF
# CO2 Panel Lernendenpaket

Version: ${VERSION}

Dieses Paket ist fuer Raspberry Pi OS mit Desktop und das 7" Raspberry Pi Display gedacht.

Installation:

\`\`\`sh
./install.sh
\`\`\`

Nach der Installation liegen die wichtigsten Dateien hier:

- UI: \`/opt/co2-panel/bin/co2_panel_ui\`
- C-Header: \`/opt/co2-panel/include/co2_panel.h\`
- C-Library: \`/opt/co2-panel/lib/libco2_panel_c_api.a\`

Die UI wird fuer den aktuellen Desktop-Benutzer automatisch in den Autostart eingetragen.
EOF

tar -C "${DIST_DIR}" -czf "${DIST_DIR}/${PACKAGE_NAME}.tar.gz" "${PACKAGE_NAME}"
echo "${DIST_DIR}/${PACKAGE_NAME}.tar.gz"
