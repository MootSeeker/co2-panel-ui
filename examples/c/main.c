#include <stdio.h>
#include <unistd.h>

#include "co2_panel.h"

static void handle_panel_event(Co2PanelEvent event)
{
    switch (event.kind) {
    case CO2_PANEL_EVENT_UNIT_PRESSED:
        printf("Taste UNIT gedrueckt, Wert: %.1f\n", event.value);
        break;
    case CO2_PANEL_EVENT_UP_PRESSED:
        printf("Taste UP gedrueckt, Wert: %.1f\n", event.value);
        break;
    case CO2_PANEL_EVENT_DOWN_PRESSED:
        printf("Taste DOWN gedrueckt, Wert: %.1f\n", event.value);
        break;
    case CO2_PANEL_EVENT_BUZZER_PRESSED:
        printf("Taste BUZZER gedrueckt, Wert: %.1f\n", event.value);
        break;
    case CO2_PANEL_EVENT_UNIT_SYSTEM_CHANGED:
        printf("Einheit gewechselt, Wert: %.1f\n", event.value);
        break;
    case CO2_PANEL_EVENT_BUZZER_CHANGED:
        printf("Buzzer-Zustand gewechselt, Wert: %.1f\n", event.value);
        break;
    case CO2_PANEL_EVENT_BRIGHTNESS_CHANGED:
        printf("Helligkeit gewechselt, Wert: %.1f\n", event.value);
        break;
    }
}

int main(void)
{
    Co2PanelConfig config = co2_panel_default_config();
    config.app_name = "CO2 Lernpanel";
    config.update_interval_ms = 1000;
    config.co2.warn = 800.0f;
    config.co2.alarm = 1200.0f;
    config.temperature.warn = 28.0f;
    config.temperature.alarm = 35.0f;
    config.humidity.warn = 65.0f;
    config.humidity.alarm = 80.0f;
    config.pressure.warn = 1030.0f;
    config.pressure.alarm = 1050.0f;

    Co2Panel *panel = co2_panel_create(&config);
    if (panel == NULL) {
        fprintf(stderr, "UI ist nicht erreichbar. Laeuft co2_panel_ui?\n");
        return 1;
    }

    for (int i = 0; i < 120; i++) {
        float co2 = 620.0f + (float)(i * 9);
        float humidity = 45.0f + (float)(i % 10);
        float temperature = 22.0f + (float)(i % 8) * 0.2f;
        float pressure = 1012.0f + (float)(i % 6);

        co2_panel_set_value(panel, CO2_PANEL_VALUE_CO2, co2);
        co2_panel_set_value(panel, CO2_PANEL_VALUE_HUMIDITY, humidity);
        co2_panel_set_value(panel, CO2_PANEL_VALUE_TEMPERATURE, temperature);
        co2_panel_set_value(panel, CO2_PANEL_VALUE_PRESSURE, pressure);

        Co2PanelEvent event;
        while (co2_panel_poll_event(panel, &event) == CO2_PANEL_OK) {
            handle_panel_event(event);
        }

        sleep(1);
    }

    co2_panel_destroy(panel);
    return 0;
}

