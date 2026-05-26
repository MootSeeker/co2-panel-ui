use std::collections::VecDeque;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;

use chrono::Local;
use co2_panel_protocol::{
    ClientMessage, EventKind, PanelConfig, PanelEvent, ServerMessage, UnitSystem, ValueKind,
};
use eframe::egui::{
    self, Align, Color32, FontId, Layout, Margin, RichText, Stroke, Vec2, ViewportCommand,
};

const BACKLIGHT_BRIGHTNESS_PATH: &str = "/sys/class/backlight/rpi_backlight/brightness";
const BACKLIGHT_MAX_PATH: &str = "/sys/class/backlight/rpi_backlight/max_brightness";

fn main() -> eframe::Result<()> {
    let state = Arc::new(Mutex::new(AppState::default()));
    let socket_path = {
        let state = state.lock().expect("state lock poisoned");
        state.config.socket_path.clone()
    };
    start_socket_server(socket_path, state.clone());

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("CO2 Panel")
            .with_inner_size([800.0, 480.0])
            .with_min_inner_size([800.0, 480.0])
            .with_fullscreen(true),
        ..Default::default()
    };

    eframe::run_native(
        "CO2 Panel",
        native_options,
        Box::new(|cc| {
            setup_style(cc);
            Ok(Box::new(Co2PanelApp { state }))
        }),
    )
}

struct Co2PanelApp {
    state: Arc<Mutex<AppState>>,
}

#[derive(Clone, Copy, Debug)]
enum EditTarget {
    Units,
    Brightness,
}

#[derive(Debug)]
struct AppState {
    config: PanelConfig,
    values: Measurements,
    events: VecDeque<PanelEvent>,
    unit_system: UnitSystem,
    buzzer_enabled: bool,
    brightness_percent: u8,
    edit_target: EditTarget,
}

#[derive(Clone, Copy, Debug, Default)]
struct Measurements {
    co2: Option<f32>,
    humidity: Option<f32>,
    temperature: Option<f32>,
    pressure: Option<f32>,
}

impl Default for AppState {
    fn default() -> Self {
        let config = PanelConfig::default();
        Self {
            unit_system: config.unit_system,
            buzzer_enabled: false,
            brightness_percent: config.brightness_percent,
            config,
            values: Measurements::default(),
            events: VecDeque::new(),
            edit_target: EditTarget::Units,
        }
    }
}

impl eframe::App for Co2PanelApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.send_viewport_cmd(ViewportCommand::Fullscreen(true));
        ctx.request_repaint_after(std::time::Duration::from_millis(500));

        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(Color32::from_rgb(238, 241, 236)))
            .show(ctx, |ui| {
                ui.set_min_size(Vec2::new(800.0, 480.0));

                let mut state = self.state.lock().expect("state lock poisoned");
                draw_header(ui, &mut state);
                draw_measurements(ui, &state);
            });
    }
}

fn setup_style(cc: &eframe::CreationContext<'_>) {
    let mut style = (*cc.egui_ctx.style()).clone();
    style.spacing.button_padding = Vec2::new(10.0, 8.0);
    style.visuals.widgets.inactive.bg_fill = Color32::from_rgb(222, 228, 220);
    style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(207, 218, 206);
    style.visuals.widgets.active.bg_fill = Color32::from_rgb(188, 205, 190);
    cc.egui_ctx.set_style(style);
}

fn draw_header(ui: &mut egui::Ui, state: &mut AppState) {
    let now = Local::now();
    let time = now.format("%H:%M").to_string();
    let date = now.format("%d.%b\n%Y").to_string().to_uppercase();

    egui::Frame::default()
        .inner_margin(Margin::symmetric(14.0, 8.0))
        .show(ui, |ui| {
            ui.allocate_ui_with_layout(
                Vec2::new(ui.available_width(), 96.0),
                Layout::left_to_right(Align::Center),
                |ui| {
                    ui.label(
                        RichText::new(time)
                            .font(FontId::proportional(86.0))
                            .strong()
                            .color(Color32::from_rgb(12, 43, 132)),
                    );
                    ui.add_space(14.0);
                    ui.label(
                        RichText::new(date)
                            .font(FontId::proportional(32.0))
                            .strong()
                            .color(Color32::from_rgb(21, 44, 117)),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.add_space(4.0);
                        draw_button_column(ui, state);
                    });
                },
            );
        });
    ui.separator();
}

fn draw_button_column(ui: &mut egui::Ui, state: &mut AppState) {
    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            if control_button(ui, "UNIT").clicked() {
                state.edit_target = match state.edit_target {
                    EditTarget::Units => EditTarget::Brightness,
                    EditTarget::Brightness => EditTarget::Units,
                };
                let value = target_value(state);
                push_event(state, EventKind::UnitPressed, value);
            }
            if control_button(ui, "BUZZER").clicked() {
                state.buzzer_enabled = !state.buzzer_enabled;
                let value = bool_value(state.buzzer_enabled);
                push_event(state, EventKind::BuzzerPressed, value);
                push_event(state, EventKind::BuzzerChanged, value);
            }
            if control_button(ui, "UP").clicked() {
                adjust_selected_setting(state, 1);
                let value = target_value(state);
                push_event(state, EventKind::UpPressed, value);
            }
            if control_button(ui, "DOWN").clicked() {
                adjust_selected_setting(state, -1);
                let value = target_value(state);
                push_event(state, EventKind::DownPressed, value);
            }
        });
        ui.add_space(8.0);
        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
            let mode = match state.edit_target {
                EditTarget::Units => "Einheiten",
                EditTarget::Brightness => "Helligkeit",
            };
            let units = match state.unit_system {
                UnitSystem::Metric => "Metrisch",
                UnitSystem::Imperial => "Imperial",
            };
            let buzzer = if state.buzzer_enabled {
                "Buzzer an"
            } else {
                "Buzzer aus"
            };
            ui.label(
                RichText::new(mode)
                    .font(FontId::proportional(15.0))
                    .strong(),
            );
            ui.label(RichText::new(units).font(FontId::proportional(15.0)));
            ui.label(
                RichText::new(format!("{}%", state.brightness_percent))
                    .font(FontId::proportional(15.0)),
            );
            ui.label(RichText::new(buzzer).font(FontId::proportional(15.0)));
        });
    });
}

fn control_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.add_sized(
        Vec2::new(72.0, 38.0),
        egui::Button::new(
            RichText::new(label)
                .font(FontId::proportional(15.0))
                .strong(),
        )
        .rounding(4.0),
    )
}

fn draw_measurements(ui: &mut egui::Ui, state: &AppState) {
    egui::Grid::new("measurements")
        .num_columns(2)
        .spacing(Vec2::new(0.0, 0.0))
        .show(ui, |ui| {
            measurement_tile(
                ui,
                "FEUCHTIGKEIT",
                state.values.humidity,
                "%",
                &state.config.humidity,
            );
            measurement_tile(ui, "CO2", state.values.co2, "ppm", &state.config.co2);
            ui.end_row();
            measurement_tile(
                ui,
                "TEMPERATUR",
                converted_temperature(state),
                temperature_unit(state.unit_system),
                &state.config.temperature,
            );
            measurement_tile(
                ui,
                "UMGEBUNGSDRUCK",
                converted_pressure(state),
                pressure_unit(state.unit_system),
                &state.config.pressure,
            );
        });
}

fn measurement_tile(
    ui: &mut egui::Ui,
    title: &str,
    value: Option<f32>,
    unit: &str,
    limits: &co2_panel_protocol::Limits,
) {
    let fill = match value {
        Some(value) if value >= limits.alarm => Color32::from_rgb(214, 66, 56),
        Some(value) if value >= limits.warn => Color32::from_rgb(240, 201, 69),
        _ => Color32::from_rgb(243, 246, 241),
    };

    egui::Frame::default()
        .fill(fill)
        .stroke(Stroke::new(1.0, Color32::from_rgb(75, 88, 91)))
        .inner_margin(Margin::symmetric(10.0, 10.0))
        .show(ui, |ui| {
            ui.allocate_ui_with_layout(
                Vec2::new(378.0, 168.0),
                Layout::top_down(Align::Center),
                |ui| {
                    ui.add_space(12.0);
                    let value_text = value
                        .map(|value| format!("{value:.1}"))
                        .unwrap_or_else(|| "--.-".to_string());
                    ui.label(
                        RichText::new(value_text)
                            .font(FontId::proportional(56.0))
                            .strong()
                            .color(Color32::from_rgb(21, 44, 117)),
                    );
                    ui.label(
                        RichText::new(unit)
                            .font(FontId::proportional(22.0))
                            .strong()
                            .color(Color32::from_rgb(21, 44, 117)),
                    );
                    ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
                        ui.label(
                            RichText::new(title)
                                .font(FontId::proportional(28.0))
                                .strong()
                                .color(Color32::from_rgb(54, 69, 132)),
                        );
                    });
                },
            );
        });
}

fn adjust_selected_setting(state: &mut AppState, direction: i32) {
    match state.edit_target {
        EditTarget::Units => {
            state.unit_system = match state.unit_system {
                UnitSystem::Metric => UnitSystem::Imperial,
                UnitSystem::Imperial => UnitSystem::Metric,
            };
            let value = target_value(state);
            push_event(state, EventKind::UnitSystemChanged, value);
        }
        EditTarget::Brightness => {
            let brightness = state.brightness_percent as i32 + direction * 5;
            state.brightness_percent = brightness.clamp(10, 100) as u8;
            apply_brightness(state.brightness_percent);
            let value = state.brightness_percent as f32;
            push_event(state, EventKind::BrightnessChanged, value);
        }
    }
}

fn target_value(state: &AppState) -> f32 {
    match state.edit_target {
        EditTarget::Units => match state.unit_system {
            UnitSystem::Metric => 0.0,
            UnitSystem::Imperial => 1.0,
        },
        EditTarget::Brightness => state.brightness_percent as f32,
    }
}

fn push_event(state: &mut AppState, kind: EventKind, value: f32) {
    state.events.push_back(PanelEvent { kind, value });
    while state.events.len() > 32 {
        state.events.pop_front();
    }
}

fn bool_value(value: bool) -> f32 {
    if value {
        1.0
    } else {
        0.0
    }
}

fn converted_temperature(state: &AppState) -> Option<f32> {
    state
        .values
        .temperature
        .map(|value| match state.unit_system {
            UnitSystem::Metric => value,
            UnitSystem::Imperial => value * 9.0 / 5.0 + 32.0,
        })
}

fn converted_pressure(state: &AppState) -> Option<f32> {
    state.values.pressure.map(|value| match state.unit_system {
        UnitSystem::Metric => value,
        UnitSystem::Imperial => value * 0.029_529_983,
    })
}

fn temperature_unit(unit_system: UnitSystem) -> &'static str {
    match unit_system {
        UnitSystem::Metric => "C",
        UnitSystem::Imperial => "F",
    }
}

fn pressure_unit(unit_system: UnitSystem) -> &'static str {
    match unit_system {
        UnitSystem::Metric => "hPa",
        UnitSystem::Imperial => "inHg",
    }
}

fn start_socket_server(socket_path: String, state: Arc<Mutex<AppState>>) {
    thread::spawn(move || {
        if Path::new(&socket_path).exists() {
            let _ = fs::remove_file(&socket_path);
        }

        let listener = match UnixListener::bind(&socket_path) {
            Ok(listener) => listener,
            Err(error) => {
                eprintln!("Cannot bind socket {socket_path}: {error}");
                return;
            }
        };

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let state = state.clone();
                    thread::spawn(move || handle_client(stream, state));
                }
                Err(error) => eprintln!("Cannot accept UI client: {error}"),
            }
        }
    });
}

fn handle_client(mut stream: UnixStream, state: Arc<Mutex<AppState>>) {
    let reader_stream = match stream.try_clone() {
        Ok(stream) => stream,
        Err(error) => {
            eprintln!("Cannot clone client stream: {error}");
            return;
        }
    };
    let reader = BufReader::new(reader_stream);

    for line in reader.lines() {
        let response = match line {
            Ok(line) => handle_message(&line, &state),
            Err(error) => ServerMessage::Error {
                message: format!("Cannot read request: {error}"),
            },
        };

        let response = match co2_panel_protocol::encode_line(&response) {
            Ok(response) => response,
            Err(error) => {
                format!(r#"{{"type":"error","message":"Cannot encode response: {error}"}}"#) + "\n"
            }
        };

        if stream.write_all(response.as_bytes()).is_err() {
            return;
        }
    }
}

fn handle_message(line: &str, state: &Arc<Mutex<AppState>>) -> ServerMessage {
    let message = match serde_json::from_str::<ClientMessage>(line) {
        Ok(message) => message,
        Err(error) => {
            return ServerMessage::Error {
                message: format!("Cannot decode request: {error}"),
            }
        }
    };

    let mut state = state.lock().expect("state lock poisoned");
    match message {
        ClientMessage::Configure { config } => {
            state.config = config.clone();
            state.unit_system = config.unit_system;
            state.brightness_percent = config.brightness_percent;
            apply_brightness(state.brightness_percent);
            ServerMessage::Ok
        }
        ClientMessage::SetValue { kind, value } => {
            set_value(&mut state.values, kind, value);
            ServerMessage::Ok
        }
        ClientMessage::GetValue { kind } => ServerMessage::Value {
            kind,
            value: get_value(&state.values, kind),
        },
        ClientMessage::GetEvent => ServerMessage::Event {
            event: state.events.pop_front(),
        },
    }
}

fn set_value(values: &mut Measurements, kind: ValueKind, value: f32) {
    match kind {
        ValueKind::Co2 => values.co2 = Some(value),
        ValueKind::Humidity => values.humidity = Some(value),
        ValueKind::Temperature => values.temperature = Some(value),
        ValueKind::Pressure => values.pressure = Some(value),
    }
}

fn get_value(values: &Measurements, kind: ValueKind) -> Option<f32> {
    match kind {
        ValueKind::Co2 => values.co2,
        ValueKind::Humidity => values.humidity,
        ValueKind::Temperature => values.temperature,
        ValueKind::Pressure => values.pressure,
    }
}

fn apply_brightness(percent: u8) {
    if write_raspberry_pi_backlight(percent).is_err() {
        let brightness = (percent as f32 / 100.0).clamp(0.1, 1.0).to_string();
        let _ = Command::new("xrandr")
            .args(["--output", "DSI-1", "--brightness", &brightness])
            .status();
    }
}

fn write_raspberry_pi_backlight(percent: u8) -> Result<(), std::io::Error> {
    let max_brightness: u32 = fs::read_to_string(BACKLIGHT_MAX_PATH)?
        .trim()
        .parse()
        .unwrap_or(255);
    let brightness = ((max_brightness as f32) * (percent as f32 / 100.0)).round() as u32;
    fs::write(BACKLIGHT_BRIGHTNESS_PATH, brightness.to_string())
}
