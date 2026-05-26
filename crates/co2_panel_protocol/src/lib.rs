use serde::{Deserialize, Serialize};

pub const DEFAULT_SOCKET_PATH: &str = "/tmp/co2-panel.sock";

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ValueKind {
    Co2,
    Humidity,
    Temperature,
    Pressure,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UnitSystem {
    Metric,
    Imperial,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    UnitPressed,
    UpPressed,
    DownPressed,
    BuzzerPressed,
    UnitSystemChanged,
    BuzzerChanged,
    BrightnessChanged,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Limits {
    pub warn: f32,
    pub alarm: f32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PanelConfig {
    pub app_name: String,
    pub socket_path: String,
    pub fullscreen: bool,
    pub update_interval_ms: u32,
    pub unit_system: UnitSystem,
    pub brightness_percent: u8,
    pub co2: Limits,
    pub humidity: Limits,
    pub temperature: Limits,
    pub pressure: Limits,
}

impl Default for PanelConfig {
    fn default() -> Self {
        Self {
            app_name: "CO2 Panel".to_string(),
            socket_path: DEFAULT_SOCKET_PATH.to_string(),
            fullscreen: true,
            update_interval_ms: 1000,
            unit_system: UnitSystem::Metric,
            brightness_percent: 80,
            co2: Limits {
                warn: 800.0,
                alarm: 1200.0,
            },
            humidity: Limits {
                warn: 65.0,
                alarm: 80.0,
            },
            temperature: Limits {
                warn: 28.0,
                alarm: 35.0,
            },
            pressure: Limits {
                warn: 1030.0,
                alarm: 1050.0,
            },
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    Configure { config: PanelConfig },
    SetValue { kind: ValueKind, value: f32 },
    GetValue { kind: ValueKind },
    GetEvent,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Ok,
    Value { kind: ValueKind, value: Option<f32> },
    Event { event: Option<PanelEvent> },
    Error { message: String },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PanelEvent {
    pub kind: EventKind,
    pub value: f32,
}

pub fn encode_line<T: Serialize>(message: &T) -> Result<String, serde_json::Error> {
    let mut line = serde_json::to_string(message)?;
    line.push('\n');
    Ok(line)
}

