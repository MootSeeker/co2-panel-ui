# CO2 Panel fuer Raspberry Pi

Dieses Projekt stellt ein UI-Programm fuer Raspberry Pi 4 und Raspberry Pi 5 bereit. Lernende koennen aus einem C-Programm Messwerte setzen, ohne selbst eine grafische Oberflaeche programmieren zu muessen.

Das Zielsystem ist Raspberry Pi OS mit Desktop und einem 7" Raspberry Pi Display im Querformat mit 800x480 px.

## Bestandteile

- `crates/co2_panel_ui`: Rust-Vollbild-UI fuer das Display.
- `crates/co2_panel_c_api`: statische Rust-Library mit C-ABI.
- `include/co2_panel.h`: Header-Datei fuer Lernendenprogramme.
- `examples/c`: kleines C-Beispielprogramm.
- `docs/API.md`: deutsche API-Dokumentation.

## Bauen

Rust installieren:

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Projekt bauen:

```sh
cargo build --release
```

## UI starten

```sh
cargo run --release -p co2_panel_ui
```

Die UI oeffnet sich im Vollbild und legt den lokalen Socket `/tmp/co2-panel.sock` an.

## C-Beispiel bauen und starten

In einem zweiten Terminal:

```sh
cd examples/c
make
./demo
```

Das Beispiel aktualisiert jede Sekunde CO2, Feuchtigkeit, Temperatur und Umgebungsdruck.

## Autostart auf Raspberry Pi OS

Fuer den Desktop-Autostart kann eine `.desktop` Datei unter `~/.config/autostart/` abgelegt werden. Eine Vorlage liegt in `deploy/co2-panel.desktop.template`.

Vorher muss der absolute Pfad zum gebauten Programm angepasst werden.

## Helligkeit

Die UI versucht zuerst, die Helligkeit ueber `/sys/class/backlight/rpi_backlight/brightness` zu setzen. Falls das nicht moeglich ist, wird als Fallback `xrandr --output DSI-1 --brightness` verwendet.

Je nach Raspberry Pi OS Konfiguration koennen fuer die direkte Backlight-Steuerung zusaetzliche Rechte noetig sein.

