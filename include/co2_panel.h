#ifndef CO2_PANEL_H
#define CO2_PANEL_H

#ifdef __cplusplus
extern "C" {
#endif

#include <stdint.h>

typedef struct Co2Panel Co2Panel;

typedef enum {
    CO2_PANEL_OK = 0,
    CO2_PANEL_ERROR = -1,
    CO2_PANEL_NO_EVENT = 1
} Co2PanelStatus;

typedef enum {
    CO2_PANEL_VALUE_CO2 = 0,
    CO2_PANEL_VALUE_HUMIDITY = 1,
    CO2_PANEL_VALUE_TEMPERATURE = 2,
    CO2_PANEL_VALUE_PRESSURE = 3
} Co2PanelValueKind;

typedef enum {
    CO2_PANEL_UNIT_METRIC = 0,
    CO2_PANEL_UNIT_IMPERIAL = 1
} Co2PanelUnitSystem;

typedef enum {
    CO2_PANEL_EVENT_UNIT_PRESSED = 0,
    CO2_PANEL_EVENT_UP_PRESSED = 1,
    CO2_PANEL_EVENT_DOWN_PRESSED = 2,
    CO2_PANEL_EVENT_BUZZER_PRESSED = 3,
    CO2_PANEL_EVENT_UNIT_SYSTEM_CHANGED = 4,
    CO2_PANEL_EVENT_BUZZER_CHANGED = 5,
    CO2_PANEL_EVENT_BRIGHTNESS_CHANGED = 6
} Co2PanelEventKind;

typedef struct {
    float warn;
    float alarm;
} Co2PanelLimits;

typedef struct {
    const char *app_name;
    const char *socket_path;
    uint8_t fullscreen;
    uint32_t update_interval_ms;
    Co2PanelUnitSystem unit_system;
    uint8_t brightness_percent;
    Co2PanelLimits co2;
    Co2PanelLimits humidity;
    Co2PanelLimits temperature;
    Co2PanelLimits pressure;
} Co2PanelConfig;

typedef struct {
    Co2PanelEventKind kind;
    float value;
} Co2PanelEvent;

Co2PanelConfig co2_panel_default_config(void);
Co2Panel *co2_panel_create(const Co2PanelConfig *config);
void co2_panel_destroy(Co2Panel *panel);

Co2PanelStatus co2_panel_set_value(Co2Panel *panel, Co2PanelValueKind kind, float value);
Co2PanelStatus co2_panel_get_value(Co2Panel *panel, Co2PanelValueKind kind, float *value);
Co2PanelStatus co2_panel_poll_event(Co2Panel *panel, Co2PanelEvent *event);
const char *co2_panel_last_error(Co2Panel *panel);

#ifdef __cplusplus
}
#endif

#endif

