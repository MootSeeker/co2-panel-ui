# CO2 Panel C-API

Diese Datei beschreibt die Schnittstelle, die Lernende aus einem C-Programm verwenden.
Die UI-Arbeit wird von der Rust-Anwendung `co2_panel_ui` erledigt. Das C-Programm setzt nur Werte, liest optional Werte zurueck und fragt Touch-Ereignisse ab.

## Architektur

- `co2_panel_ui`: Rust-Programm mit grafischer Vollbildoberflaeche fuer das 7" Raspberry Pi Display.
- `co2_panel_c_api`: Rust-Library mit C-kompatibler ABI.
- `include/co2_panel.h`: Header-Datei fuer C-Projekte.
- Kommunikation: lokaler Unix Socket, standardmaessig `/tmp/co2-panel.sock`.

Die UI muss laufen, bevor das C-Programm `co2_panel_create()` aufruft.

## Grundablauf

```c
#include "co2_panel.h"

Co2PanelConfig config = co2_panel_default_config();
Co2Panel *panel = co2_panel_create(&config);

co2_panel_set_value(panel, CO2_PANEL_VALUE_CO2, 820.0f);
co2_panel_set_value(panel, CO2_PANEL_VALUE_TEMPERATURE, 22.5f);
co2_panel_set_value(panel, CO2_PANEL_VALUE_HUMIDITY, 48.0f);
co2_panel_set_value(panel, CO2_PANEL_VALUE_PRESSURE, 1012.0f);

Co2PanelEvent event;
if (co2_panel_poll_event(panel, &event) == CO2_PANEL_OK) {
    /* Touch-Ereignis auswerten */
}

co2_panel_destroy(panel);
```

## Messwerte

Alle Messwerte werden als `float` uebergeben.

| Messwert | Enum | Erwartete Einheit im C-Programm |
| --- | --- | --- |
| CO2 | `CO2_PANEL_VALUE_CO2` | ppm |
| Feuchtigkeit | `CO2_PANEL_VALUE_HUMIDITY` | % |
| Temperatur | `CO2_PANEL_VALUE_TEMPERATURE` | Grad Celsius |
| Umgebungsdruck | `CO2_PANEL_VALUE_PRESSURE` | hPa |

Das UI kann die Anzeige zwischen metrischen und imperialen Einheiten wechseln. Das C-Programm liefert trotzdem immer die metrischen Basiswerte.

## Konfiguration

Die Standardkonfiguration wird mit `co2_panel_default_config()` erzeugt. Danach koennen einzelne Felder angepasst werden.

```c
Co2PanelConfig config = co2_panel_default_config();
config.app_name = "CO2 Lernpanel";
config.update_interval_ms = 1000;
config.brightness_percent = 80;
config.co2.warn = 800.0f;
config.co2.alarm = 1200.0f;
```

Wichtige Felder:

| Feld | Bedeutung |
| --- | --- |
| `app_name` | Name der Anwendung |
| `socket_path` | Pfad zum lokalen Unix Socket |
| `fullscreen` | `1` fuer Vollbild |
| `update_interval_ms` | Empfohlenes Update-Intervall |
| `unit_system` | Startwert fuer metrisch oder imperial |
| `brightness_percent` | Display-Helligkeit in Prozent |
| `co2`, `humidity`, `temperature`, `pressure` | Warn- und Alarmgrenzen |

Warnwerte werden gelb dargestellt. Alarmwerte werden rot dargestellt.

## Touch-Ereignisse

Die UI stellt folgende Ereignisse bereit:

| Ereignis | Bedeutung |
| --- | --- |
| `CO2_PANEL_EVENT_UNIT_PRESSED` | Taste `UNIT` wurde gedrueckt |
| `CO2_PANEL_EVENT_UP_PRESSED` | Taste `UP` wurde gedrueckt |
| `CO2_PANEL_EVENT_DOWN_PRESSED` | Taste `DOWN` wurde gedrueckt |
| `CO2_PANEL_EVENT_BUZZER_PRESSED` | Taste `BUZZER` wurde gedrueckt |
| `CO2_PANEL_EVENT_UNIT_SYSTEM_CHANGED` | Einheitensystem wurde geaendert |
| `CO2_PANEL_EVENT_BUZZER_CHANGED` | Visueller Buzzer-Zustand wurde geaendert |
| `CO2_PANEL_EVENT_BRIGHTNESS_CHANGED` | Helligkeit wurde geaendert |

`event.value` enthaelt je nach Ereignis einen Zusatzwert:

- Einheit: `0.0` fuer metrisch, `1.0` fuer imperial.
- Buzzer: `0.0` fuer aus, `1.0` fuer an.
- Helligkeit: Prozentwert von `10.0` bis `100.0`.

## Buttons

Die Bedienung im ersten Stand:

- `UNIT`: wechselt zwischen dem Einstellmodus `Einheiten` und `Helligkeit`.
- `UP`/`DOWN` im Modus `Einheiten`: wechselt zwischen metrisch und imperial.
- `UP`/`DOWN` im Modus `Helligkeit`: aendert die Displayhelligkeit in 5-Prozent-Schritten.
- `BUZZER`: schaltet nur den visuellen Buzzer-Zustand um. Es wird kein GPIO-Pin geschaltet.

