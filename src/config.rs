use rubo_engine::{
    ConfigError, Engine,
    config::{
        AppConfig, BindingConfig, ConfigAccess, DeviceConfig, FuncConfig, RuboConfig, SinkConfig,
        SourceConfig,
    },
};

use crate::{device::GpioDevice, sink::HeadlessWebSink};

pub const UART_SOURCE_ID: &str = "uart";
pub const CAMERA_ID: &str = "camera";
pub const WEB_SINK_ID: &str = "web";
pub const UART_SINK_ID: &str = "uart";
pub const GPIO_SINK_ID: &str = "gpio";

pub fn default_app_config() -> AppConfig {
    serde_json::from_value(serde_json::json!({
        "name": "rubo_vision",
        "config_path": "config",
        "profile": "orangepi",
        "config_format": "toml",
        "web": {
            "enabled": true,
            "host": "0.0.0.0",
            "port": 3888
        },
        "log": {
            "enabled": true,
            "level": "info"
        }
    }))
    .expect("default app config must be valid")
}

pub fn default_rubo_config(app_config: &AppConfig) -> Result<RuboConfig, ConfigError> {
    let orangepi = match app_config.profile() {
        "orangepi" => true,
        "raspberrypi" => false,
        _ => {
            return Err(ConfigError::ConfigLoad {
                message: format!(
                    "unsupported config profile {}; expected orangepi or raspberrypi",
                    app_config.profile()
                ),
            });
        }
    };
    let mut config = RuboConfig::default();
    insert_sources(
        &mut config,
        if orangepi {
            "/dev/ttyAMA1"
        } else {
            "/dev/serial0"
        },
    );
    insert_devices(
        &mut config,
        if orangepi {
            "/dev/video0"
        } else {
            "/dev/video2"
        },
    );
    insert_functions(&mut config);
    insert_sinks(&mut config, orangepi);
    insert_bindings(&mut config);
    Ok(config)
}

pub fn build_engine(
    root: impl AsRef<std::path::Path>,
    app_config: AppConfig,
    mut rubo_config: RuboConfig,
) -> Engine {
    let web_enabled = app_config.web().enabled();
    if !web_enabled {
        rubo_config
            .bindings_mut()
            .retain(|_, binding| binding.source_ref().id() != WEB_SINK_ID);
    }
    let gpio = match rubo_config.sinks().get(GPIO_SINK_ID) {
        Some(config) => GpioDevice::from_config(config).unwrap_or_else(|error| {
            eprintln!(
                "{}",
                rubo_engine::log::error_text(format!(
                    "rubo_vision.gpio.config.error error={error}; gpio is disabled"
                ))
            );
            GpioDevice::default()
        }),
        None => GpioDevice::default(),
    };
    let mut engine = Engine::new(root, app_config, rubo_config);
    engine.register_sink(GPIO_SINK_ID, gpio.clone());
    engine.register_function_aspect(gpio);
    if !web_enabled {
        engine.register_sink(WEB_SINK_ID, HeadlessWebSink);
    }
    engine
}

fn insert_sources(config: &mut RuboConfig, serial: &str) {
    config.sources_mut().insert(
        UART_SOURCE_ID.to_string(),
        SourceConfig::new(UART_SOURCE_ID)
            .kind("uart")
            .set("serial", serial)
            .set("baud", 9600_u32)
            .set("data_bit", 8_u8)
            .set("stop_bit", 1_u8)
            .set("parity_bit", false)
            .set("prefix", vec![b'a'])
            .set("suffix", vec![b'\r', b'\n'])
            .set("content_bytes", 1_usize),
    );
}

fn insert_devices(config: &mut RuboConfig, path: &str) {
    config.devices_mut().insert(
        CAMERA_ID.to_string(),
        DeviceConfig::new(CAMERA_ID, "camera").set("path", path),
    );
}

fn insert_functions(config: &mut RuboConfig) {
    config.funcs_mut().insert(
        "color_detect".to_string(),
        FuncConfig::new("color_detect")
            .set("device_id", CAMERA_ID)
            .set("max_frames", 30_usize)
            .set("confirm_frames", 5_usize)
            .set("radius_ratio", 0.4_f64)
            .set("min_area_ratio", 0.8_f64)
            .set("colors", default_colors()),
    );
    config.funcs_mut().insert(
        "qr_detect".to_string(),
        FuncConfig::new("qr_detect")
            .set("device_id", CAMERA_ID)
            .set("max_frames", 30_usize),
    );
    config.funcs_mut().insert(
        "letter_detect".to_string(),
        FuncConfig::new("letter_detect")
            .set("device_id", CAMERA_ID)
            .set("max_frames", 30_usize)
            .set("confirm_frames", 3_usize)
            .set("black_threshold", 90_i32)
            .set("min_letter_area_ratio", 0.05_f64),
    );
    config.funcs_mut().insert(
        "black_ring_detect".to_string(),
        FuncConfig::new("black_ring_detect")
            .set("device_id", CAMERA_ID)
            .set("max_frames", 3_usize)
            .set("target_correction", default_target_correction())
            .set("black_threshold", 90_i32)
            .set("min_radius", 20.0_f64)
            .set("max_radius", 180.0_f64)
            .set("min_circularity", 0.65_f64)
            .set("min_score", 50_u8),
    );
    config.funcs_mut().insert(
        "cross".to_string(),
        FuncConfig::new("cross")
            .set("device_id", CAMERA_ID)
            .set("max_frames", 3_usize)
            .set("target_correction", default_target_correction())
            .set("black_threshold", 90_i32)
            .set("close_kernel_size", 5_i32)
            .set("dilate_kernel_size", 3_i32)
            .set("dilate_iterations", 1_i32)
            .set("min_radius", 20.0_f64)
            .set("max_radius", 600.0_f64)
            .set("center_tolerance", 14.0_f64)
            .set("min_arc_points", 24_usize)
            .set("min_ring_score", 50_u8),
    );
    config.funcs_mut().insert(
        "debug_fun".to_string(),
        FuncConfig::new("debug_fun").set("message", "debug"),
    );
}

fn insert_sinks(config: &mut RuboConfig, orangepi: bool) {
    let serial = if orangepi {
        "/dev/ttyAMA1"
    } else {
        "/dev/serial0"
    };
    config.sinks_mut().insert(
        WEB_SINK_ID.to_string(),
        SinkConfig::new(WEB_SINK_ID).kind("web"),
    );
    config.sinks_mut().insert(
        UART_SINK_ID.to_string(),
        SinkConfig::new(UART_SINK_ID)
            .kind("uart")
            .set("serial", serial)
            .set("baud", 9600_u32)
            .set("data_bit", 8_u8)
            .set("stop_bit", 1_u8)
            .set("parity_bit", false),
    );
    config.sinks_mut().insert(
        GPIO_SINK_ID.to_string(),
        SinkConfig::new(GPIO_SINK_ID)
            .kind("gpio")
            .set("active_low", true)
            .set("chip", if orangepi { 7_u8 } else { 0_u8 })
            .set("run_pin", if orangepi { 3_u32 } else { 27_u32 })
            .set("signals", default_gpio_signals(orangepi)),
    );
}

fn insert_bindings(config: &mut RuboConfig) {
    insert_uart_binding(config, "uart_color_detect", "1", CAMERA_ID, "color_detect");
    insert_uart_binding(config, "uart_qr_detect", "2", CAMERA_ID, "qr_detect");
    insert_uart_binding(config, "uart_cross_detect", "3", CAMERA_ID, "cross");
    insert_uart_binding(
        config,
        "uart_black_ring_detect",
        "4",
        CAMERA_ID,
        "black_ring_detect",
    );
    insert_uart_binding(
        config,
        "uart_letter_detect",
        "5",
        CAMERA_ID,
        "letter_detect",
    );
    config.bindings_mut().insert(
        "debug".to_string(),
        BindingConfig::new("debug")
            .source("web", "debug")
            .func("debug_fun")
            .sink(WEB_SINK_ID)
            .debug(true),
    );
    config.bindings_mut().insert(
        "uart_debug".to_string(),
        BindingConfig::new("uart_debug")
            .source(UART_SOURCE_ID, "49")
            .func("debug_fun")
            .sink(WEB_SINK_ID)
            .sink(UART_SINK_ID)
            .debug(true),
    );
}

fn insert_uart_binding(
    config: &mut RuboConfig,
    id: &str,
    key: &str,
    device_id: &str,
    function_id: &str,
) {
    config.bindings_mut().insert(
        id.to_string(),
        BindingConfig::new(id)
            .source(UART_SOURCE_ID, key)
            .func(function_id)
            .device(device_id)
            .sink(WEB_SINK_ID)
            .sink(UART_SINK_ID)
            .debug(true),
    );
}

fn default_colors() -> Vec<serde_json::Value> {
    vec![
        color_definition(
            "red",
            vec![[0, 10, 160, 255, 110, 255], [170, 179, 160, 255, 110, 255]],
        ),
        color_definition("blue", vec![[100, 137, 124, 255, 56, 255]]),
        color_definition("green", vec![[50, 100, 91, 255, 85, 255]]),
        color_definition("black", vec![[0, 179, 0, 255, 0, 76]]),
        color_definition("white", vec![[0, 170, 0, 55, 120, 255]]),
    ]
}

fn default_target_correction() -> serde_json::Value {
    serde_json::json!({ "x": 0, "y": 0 })
}

fn color_definition(name: &str, hsv_ranges: Vec<[i32; 6]>) -> serde_json::Value {
    serde_json::json!({ "name": name, "hsv_ranges": hsv_ranges })
}

fn default_gpio_signals(orangepi: bool) -> serde_json::Value {
    if orangepi {
        serde_json::json!({ "color": 4, "qr": 5 })
    } else {
        serde_json::json!({ "color": 17, "qr": 22 })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_rubo_config_test() {
        let deployed_app = rubo_engine::config::ConfigStore::load_app_config("config").unwrap();
        assert_eq!(default_app_config(), deployed_app);
        assert_eq!(deployed_app.config_path(), std::path::Path::new("config"));
        assert_eq!(deployed_app.profile(), "orangepi");
        assert_eq!(
            deployed_app.config_dir(),
            std::path::Path::new("config/orangepi")
        );
        assert_eq!(deployed_app.web().host(), "0.0.0.0");
        assert_eq!(deployed_app.web().port(), 3888);
        assert!(!std::path::Path::new("config/source.toml").exists());

        let raspberrypi_app: AppConfig = serde_json::from_value(serde_json::json!({
            "config_path": "config",
            "profile": "raspberrypi"
        }))
        .unwrap();
        let config = default_rubo_config(&raspberrypi_app).unwrap();
        assert_eq!(
            rubo_engine::config::ConfigStore::load_active_config("config/raspberrypi").unwrap(),
            config
        );
        assert_eq!(config.devices().len(), 1);
        assert!(config.devices().contains_key(CAMERA_ID));
        assert_eq!(
            config.sinks()[GPIO_SINK_ID]
                .get_or("chip", u8::MAX)
                .unwrap(),
            0
        );
        assert_eq!(
            config.sources()[UART_SOURCE_ID]
                .get_or("serial", String::new())
                .unwrap(),
            "/dev/serial0"
        );
        assert_eq!(
            config.sources()[UART_SOURCE_ID]
                .get_or("content_bytes", 0_usize)
                .unwrap(),
            1
        );
        assert_eq!(config.bindings().len(), 7);
        for id in [
            "uart_color_detect",
            "uart_qr_detect",
            "uart_cross_detect",
            "uart_black_ring_detect",
            "uart_letter_detect",
        ] {
            assert!(config.bindings()[id].debug_enabled());
            assert_eq!(config.bindings()[id].devices(), &[CAMERA_ID.to_string()]);
        }
        let uart_letter = &config.bindings()["uart_letter_detect"];
        assert_eq!(uart_letter.source_ref().id(), UART_SOURCE_ID);
        assert_eq!(uart_letter.source_ref().event(), "5");
        assert_eq!(uart_letter.func_ref(), "letter_detect");
        assert_eq!(
            uart_letter.sinks(),
            &[WEB_SINK_ID.to_string(), UART_SINK_ID.to_string()]
        );
        let debug = &config.bindings()["debug"];
        assert_eq!(debug.source_ref().id(), "web");
        assert_eq!(debug.source_ref().event(), "debug");
        assert_eq!(debug.func_ref(), "debug_fun");
        assert_eq!(debug.sinks(), &["web".to_string()]);
        let uart_debug = &config.bindings()["uart_debug"];
        assert_eq!(uart_debug.source_ref().id(), UART_SOURCE_ID);
        assert_eq!(uart_debug.source_ref().event(), "49");
        assert_eq!(uart_debug.func_ref(), "debug_fun");
        assert_eq!(
            uart_debug.sinks(),
            &[WEB_SINK_ID.to_string(), UART_SINK_ID.to_string()]
        );

        let mut engine = build_engine(".", raspberrypi_app, config);
        engine.prepare_web();
        assert_eq!(engine.config().bindings().len(), 7);
        let runtime_config = engine.web_state().unwrap().runtime_config();
        let runtime_config = runtime_config
            .read()
            .expect("test runtime config lock poisoned");
        assert!(runtime_config.validate());
        assert_eq!(runtime_config.bindings().len(), 13);

        let orangepi_app: AppConfig = serde_json::from_value(serde_json::json!({
            "config_path": "config",
            "profile": "orangepi"
        }))
        .unwrap();
        let orangepi = default_rubo_config(&orangepi_app).unwrap();
        assert_eq!(
            rubo_engine::config::ConfigStore::load_active_config("config/orangepi").unwrap(),
            orangepi
        );
        assert_eq!(
            orangepi.sources()[UART_SOURCE_ID]
                .get_or("serial", String::new())
                .unwrap(),
            "/dev/ttyAMA1"
        );
        assert_eq!(
            orangepi.sinks()[GPIO_SINK_ID]
                .get_or("chip", u8::MAX)
                .unwrap(),
            7
        );
        assert_eq!(
            orangepi.sinks()[GPIO_SINK_ID]
                .get_or("run_pin", u32::MAX)
                .unwrap(),
            3
        );
        assert_eq!(
            orangepi.devices()[CAMERA_ID]
                .get_or("path", String::new())
                .unwrap(),
            "/dev/video0"
        );

        let invalid_app: AppConfig = serde_json::from_value(serde_json::json!({
            "config_path": "config",
            "profile": "unknown"
        }))
        .unwrap();
        assert!(default_rubo_config(&invalid_app).is_err());
    }

    #[test]
    fn headless_config_test() {
        let app_config: AppConfig = serde_json::from_value(serde_json::json!({
            "config_path": "config",
            "profile": "raspberrypi",
            "web": { "enabled": false }
        }))
        .unwrap();
        let config = default_rubo_config(&app_config).unwrap();
        let engine = build_engine(".", app_config, config);

        assert!(!engine.config().bindings().contains_key("debug"));
        assert!(engine.sinks().contains(WEB_SINK_ID));
    }
}
