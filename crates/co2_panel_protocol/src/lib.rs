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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_uses_expected_socket_and_limits() {
        let config = PanelConfig::default();

        assert_eq!(config.socket_path, DEFAULT_SOCKET_PATH);
        assert!(config.fullscreen);
        assert_eq!(config.update_interval_ms, 1000);
        assert_eq!(config.brightness_percent, 80);
        assert_eq!(config.co2.warn, 800.0);
        assert_eq!(config.co2.alarm, 1200.0);
    }

    #[test]
    fn message_encoding_is_newline_delimited_json() {
        let message = ClientMessage::SetValue {
            kind: ValueKind::Co2,
            value: 901.5,
        };

        let encoded = encode_line(&message).expect("message should encode");

        assert!(encoded.ends_with('\n'));
        assert!(encoded.contains(r#""type":"set_value""#));
        assert!(encoded.contains(r#""kind":"co2""#));
    }

    #[test]
    fn event_response_roundtrips_through_json() {
        let response = ServerMessage::Event {
            event: Some(PanelEvent {
                kind: EventKind::BrightnessChanged,
                value: 75.0,
            }),
        };

        let encoded = serde_json::to_string(&response).expect("response should encode");
        let decoded: ServerMessage =
            serde_json::from_str(&encoded).expect("response should decode");

        match decoded {
            ServerMessage::Event { event: Some(event) } => {
                assert!(matches!(event.kind, EventKind::BrightnessChanged));
                assert_eq!(event.value, 75.0);
            }
            other => panic!("unexpected response: {other:?}"),
        }
    }
}
