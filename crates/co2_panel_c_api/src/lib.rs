use std::ffi::{CStr, CString};
use std::io::{BufRead, BufReader, Write};
use std::net::Shutdown;
use std::os::raw::c_char;
use std::os::unix::net::UnixStream;
use std::ptr;
use std::time::Duration;

use co2_panel_protocol::{
    ClientMessage, EventKind, Limits, PanelConfig, PanelEvent, ServerMessage, UnitSystem, ValueKind,
    DEFAULT_SOCKET_PATH,
};

const READ_TIMEOUT_MS: u64 = 200;
const WRITE_TIMEOUT_MS: u64 = 200;
static INVALID_PANEL_POINTER: &[u8] = b"Invalid panel pointer\0";

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Co2PanelLimits {
    pub warn: f32,
    pub alarm: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Co2PanelConfig {
    pub app_name: *const c_char,
    pub socket_path: *const c_char,
    pub fullscreen: u8,
    pub update_interval_ms: u32,
    pub unit_system: Co2PanelUnitSystem,
    pub brightness_percent: u8,
    pub co2: Co2PanelLimits,
    pub humidity: Co2PanelLimits,
    pub temperature: Co2PanelLimits,
    pub pressure: Co2PanelLimits,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub enum Co2PanelStatus {
    Ok = 0,
    Error = -1,
    NoEvent = 1,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub enum Co2PanelValueKind {
    Co2 = 0,
    Humidity = 1,
    Temperature = 2,
    Pressure = 3,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub enum Co2PanelUnitSystem {
    Metric = 0,
    Imperial = 1,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub enum Co2PanelEventKind {
    UnitPressed = 0,
    UpPressed = 1,
    DownPressed = 2,
    BuzzerPressed = 3,
    UnitSystemChanged = 4,
    BuzzerChanged = 5,
    BrightnessChanged = 6,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Co2PanelEvent {
    pub kind: Co2PanelEventKind,
    pub value: f32,
}

pub struct Co2Panel {
    stream: UnixStream,
    last_error: CString,
}

#[no_mangle]
pub extern "C" fn co2_panel_default_config() -> Co2PanelConfig {
    Co2PanelConfig {
        app_name: ptr::null(),
        socket_path: ptr::null(),
        fullscreen: 1,
        update_interval_ms: 1000,
        unit_system: Co2PanelUnitSystem::Metric,
        brightness_percent: 80,
        co2: Co2PanelLimits {
            warn: 800.0,
            alarm: 1200.0,
        },
        humidity: Co2PanelLimits {
            warn: 65.0,
            alarm: 80.0,
        },
        temperature: Co2PanelLimits {
            warn: 28.0,
            alarm: 35.0,
        },
        pressure: Co2PanelLimits {
            warn: 1030.0,
            alarm: 1050.0,
        },
    }
}

#[no_mangle]
pub extern "C" fn co2_panel_create(config: *const Co2PanelConfig) -> *mut Co2Panel {
    let c_config = if config.is_null() {
        co2_panel_default_config()
    } else {
        // SAFETY: The caller provides a pointer valid for this function call.
        unsafe { *config }
    };

    match create_panel(&c_config) {
        Ok(panel) => Box::into_raw(Box::new(panel)),
        Err(message) => {
            eprintln!("co2_panel_create: {message}");
            ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn co2_panel_destroy(panel: *mut Co2Panel) {
    if panel.is_null() {
        return;
    }

    // SAFETY: The pointer was returned by co2_panel_create and is consumed here once.
    let panel = unsafe { Box::from_raw(panel) };
    let _ = panel.stream.shutdown(Shutdown::Both);
}

#[no_mangle]
pub extern "C" fn co2_panel_set_value(
    panel: *mut Co2Panel,
    kind: Co2PanelValueKind,
    value: f32,
) -> Co2PanelStatus {
    with_panel(panel, |panel| {
        let message = ClientMessage::SetValue {
            kind: kind.into(),
            value,
        };
        match send_message(panel, &message) {
            Ok(ServerMessage::Ok) => Co2PanelStatus::Ok,
            Ok(response) => set_error(panel, format!("Unexpected response: {response:?}")),
            Err(error) => set_error(panel, error),
        }
    })
}

#[no_mangle]
pub extern "C" fn co2_panel_get_value(
    panel: *mut Co2Panel,
    kind: Co2PanelValueKind,
    value: *mut f32,
) -> Co2PanelStatus {
    if value.is_null() {
        return Co2PanelStatus::Error;
    }

    with_panel(panel, |panel| {
        let message = ClientMessage::GetValue { kind: kind.into() };
        match send_message(panel, &message) {
            Ok(ServerMessage::Value {
                value: Some(read_value),
                ..
            }) => {
                // SAFETY: Null was checked above, and the caller owns the output location.
                unsafe { *value = read_value };
                Co2PanelStatus::Ok
            }
            Ok(ServerMessage::Value { value: None, .. }) => {
                set_error(panel, "Value is not available yet")
            }
            Ok(response) => set_error(panel, format!("Unexpected response: {response:?}")),
            Err(error) => set_error(panel, error),
        }
    })
}

#[no_mangle]
pub extern "C" fn co2_panel_poll_event(
    panel: *mut Co2Panel,
    event: *mut Co2PanelEvent,
) -> Co2PanelStatus {
    if event.is_null() {
        return Co2PanelStatus::Error;
    }

    with_panel(panel, |panel| match send_message(panel, &ClientMessage::GetEvent) {
        Ok(ServerMessage::Event { event: Some(read_event) }) => {
            // SAFETY: Null was checked above, and the caller owns the output location.
            unsafe { *event = read_event.into() };
            Co2PanelStatus::Ok
        }
        Ok(ServerMessage::Event { event: None }) => Co2PanelStatus::NoEvent,
        Ok(response) => set_error(panel, format!("Unexpected response: {response:?}")),
        Err(error) => set_error(panel, error),
    })
}

#[no_mangle]
pub extern "C" fn co2_panel_last_error(panel: *mut Co2Panel) -> *const c_char {
    if panel.is_null() {
        return INVALID_PANEL_POINTER.as_ptr().cast();
    }

    // SAFETY: Null was checked above; returned pointer remains valid while panel exists.
    unsafe { (*panel).last_error.as_ptr() }
}

fn create_panel(config: &Co2PanelConfig) -> Result<Co2Panel, String> {
    let panel_config = panel_config_from_c(config)?;
    let stream = UnixStream::connect(&panel_config.socket_path)
        .map_err(|error| format!("Cannot connect to UI socket: {error}"))?;
    stream
        .set_read_timeout(Some(Duration::from_millis(READ_TIMEOUT_MS)))
        .map_err(|error| format!("Cannot set read timeout: {error}"))?;
    stream
        .set_write_timeout(Some(Duration::from_millis(WRITE_TIMEOUT_MS)))
        .map_err(|error| format!("Cannot set write timeout: {error}"))?;

    let mut panel = Co2Panel {
        stream,
        last_error: empty_c_string(),
    };

    match send_message(
        &mut panel,
        &ClientMessage::Configure {
            config: panel_config,
        },
    ) {
        Ok(ServerMessage::Ok) => Ok(panel),
        Ok(response) => Err(format!("Unexpected configure response: {response:?}")),
        Err(error) => Err(error),
    }
}

fn send_message(panel: &mut Co2Panel, message: &ClientMessage) -> Result<ServerMessage, String> {
    let line = co2_panel_protocol::encode_line(message)
        .map_err(|error| format!("Cannot encode message: {error}"))?;
    panel
        .stream
        .write_all(line.as_bytes())
        .map_err(|error| format!("Cannot write to UI: {error}"))?;
    panel
        .stream
        .flush()
        .map_err(|error| format!("Cannot flush UI socket: {error}"))?;

    let cloned = panel
        .stream
        .try_clone()
        .map_err(|error| format!("Cannot clone UI socket: {error}"))?;
    let mut reader = BufReader::new(cloned);
    let mut response = String::new();
    reader
        .read_line(&mut response)
        .map_err(|error| format!("Cannot read UI response: {error}"))?;

    serde_json::from_str(&response).map_err(|error| format!("Cannot decode UI response: {error}"))
}

fn with_panel<F>(panel: *mut Co2Panel, callback: F) -> Co2PanelStatus
where
    F: FnOnce(&mut Co2Panel) -> Co2PanelStatus,
{
    if panel.is_null() {
        return Co2PanelStatus::Error;
    }

    // SAFETY: Null was checked above; the C caller must not use this pointer concurrently.
    let panel = unsafe { &mut *panel };
    callback(panel)
}

fn panel_config_from_c(config: &Co2PanelConfig) -> Result<PanelConfig, String> {
    let default = PanelConfig::default();
    Ok(PanelConfig {
        app_name: c_string_or_default(config.app_name, &default.app_name)?,
        socket_path: c_string_or_default(config.socket_path, DEFAULT_SOCKET_PATH)?,
        fullscreen: config.fullscreen != 0,
        update_interval_ms: config.update_interval_ms.max(250),
        unit_system: config.unit_system.into(),
        brightness_percent: config.brightness_percent.min(100),
        co2: limits_from_c(config.co2),
        humidity: limits_from_c(config.humidity),
        temperature: limits_from_c(config.temperature),
        pressure: limits_from_c(config.pressure),
    })
}

fn c_string_or_default(value: *const c_char, default: &str) -> Result<String, String> {
    if value.is_null() {
        return Ok(default.to_string());
    }

    // SAFETY: The caller provides a valid null-terminated string pointer.
    unsafe { CStr::from_ptr(value) }
        .to_str()
        .map(|value| value.to_string())
        .map_err(|error| format!("Invalid UTF-8 string: {error}"))
}

fn limits_from_c(limits: Co2PanelLimits) -> Limits {
    Limits {
        warn: limits.warn,
        alarm: limits.alarm,
    }
}

fn set_error(panel: &mut Co2Panel, message: impl Into<String>) -> Co2PanelStatus {
    panel.last_error = safe_c_string(message.into());
    Co2PanelStatus::Error
}

fn safe_c_string(message: String) -> CString {
    CString::new(message.replace('\0', " ")).unwrap_or_else(|_| empty_c_string())
}

fn empty_c_string() -> CString {
    CString::new("").expect("empty string has no interior nul")
}

impl From<Co2PanelValueKind> for ValueKind {
    fn from(kind: Co2PanelValueKind) -> Self {
        match kind {
            Co2PanelValueKind::Co2 => ValueKind::Co2,
            Co2PanelValueKind::Humidity => ValueKind::Humidity,
            Co2PanelValueKind::Temperature => ValueKind::Temperature,
            Co2PanelValueKind::Pressure => ValueKind::Pressure,
        }
    }
}

impl From<Co2PanelUnitSystem> for UnitSystem {
    fn from(unit_system: Co2PanelUnitSystem) -> Self {
        match unit_system {
            Co2PanelUnitSystem::Metric => UnitSystem::Metric,
            Co2PanelUnitSystem::Imperial => UnitSystem::Imperial,
        }
    }
}

impl From<EventKind> for Co2PanelEventKind {
    fn from(kind: EventKind) -> Self {
        match kind {
            EventKind::UnitPressed => Co2PanelEventKind::UnitPressed,
            EventKind::UpPressed => Co2PanelEventKind::UpPressed,
            EventKind::DownPressed => Co2PanelEventKind::DownPressed,
            EventKind::BuzzerPressed => Co2PanelEventKind::BuzzerPressed,
            EventKind::UnitSystemChanged => Co2PanelEventKind::UnitSystemChanged,
            EventKind::BuzzerChanged => Co2PanelEventKind::BuzzerChanged,
            EventKind::BrightnessChanged => Co2PanelEventKind::BrightnessChanged,
        }
    }
}

impl From<PanelEvent> for Co2PanelEvent {
    fn from(event: PanelEvent) -> Self {
        Self {
            kind: event.kind.into(),
            value: event.value,
        }
    }
}
