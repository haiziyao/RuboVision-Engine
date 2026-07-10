use rubo_engine::{
    Engine,
    config::{
        AppConfig, BindingConfig, ConfigAccess, DeviceConfig, FuncConfig, RuboConfig, SinkConfig,
        SourceConfig,
    },
};

use crate::sink::{GpioSink, HeadlessWebSink};

pub const UART_SOURCE_ID: &str = "uart";
pub const COLOR_CAMERA_ID: &str = "color_camera";
pub const QR_CAMERA_ID: &str = "qr_camera";
pub const CROSS_CAMERA_ID: &str = "cross_camera";
pub const WEB_SINK_ID: &str = "web";
pub const UART_SINK_ID: &str = "uart";
pub const GPIO_SINK_ID: &str = "gpio";

pub fn default_app_config() -> AppConfig {
    AppConfig::default()
}

pub fn default_rubo_config() -> RuboConfig {
    let mut config = RuboConfig::default();
    insert_sources(&mut config);
    insert_devices(&mut config);
    insert_functions(&mut config);
    insert_sinks(&mut config);
    insert_bindings(&mut config);
    config
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
    let gpio = rubo_config
        .sinks()
        .get(GPIO_SINK_ID)
        .map(GpioSink::from_config)
        .unwrap_or_default();
    let mut engine = Engine::new(root, app_config, rubo_config);
    engine.register_function_aspect(gpio);
    if !web_enabled {
        engine.register_sink(WEB_SINK_ID, HeadlessWebSink);
    }
    engine
}

fn insert_sources(config: &mut RuboConfig) {
    config.sources_mut().insert(
        UART_SOURCE_ID.to_string(),
        SourceConfig::new(UART_SOURCE_ID)
            .kind("uart")
            .set("serial", "/dev/ttyV0")
            .set("baud", 9600_u32)
            .set("data_bit", 8_u8)
            .set("stop_bit", 1_u8)
            .set("parity_bit", false),
    );
}

fn insert_devices(config: &mut RuboConfig) {
    config.devices_mut().insert(
        COLOR_CAMERA_ID.to_string(),
        DeviceConfig::new(COLOR_CAMERA_ID, "camera").set("path", "/dev/video2"),
    );
    config.devices_mut().insert(
        QR_CAMERA_ID.to_string(),
        DeviceConfig::new(QR_CAMERA_ID, "camera").set("path", "/dev/video2"),
    );
    config.devices_mut().insert(
        CROSS_CAMERA_ID.to_string(),
        DeviceConfig::new(CROSS_CAMERA_ID, "camera").set("path", "/dev/video2"),
    );
}

fn insert_functions(config: &mut RuboConfig) {
    config.funcs_mut().insert(
        "color_detect".to_string(),
        FuncConfig::new("color_detect")
            .set("device_id", COLOR_CAMERA_ID)
            .set("debug_model", false)
            .set("loop_count", 5_i32)
            .set("radius_ratio", 0.4_f64)
            .set("detect_area_access_rate", 0.8_f64)
            .set("color_ranges", default_color_ranges()),
    );
    config.funcs_mut().insert(
        "qr_detect".to_string(),
        FuncConfig::new("qr_detect")
            .set("device_id", QR_CAMERA_ID)
            .set("debug_model", false)
            .set("loop_count", 30_i32),
    );
    config.funcs_mut().insert(
        "black_ring_detect".to_string(),
        FuncConfig::new("black_ring_detect")
            .set("device_id", COLOR_CAMERA_ID)
            .set("debug_model", false)
            .set("loop_count", 3_i32)
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
            .set("device_id", CROSS_CAMERA_ID)
            .set("debug_model", false)
            .set("loop_count", 3_i32)
            .set("target_correction", default_target_correction())
            .set("black_threshold", 90_i32)
            .set("close_kernel_size", 5_i32)
            .set("dilate_kernel_size", 3_i32)
            .set("dilate_iterations", 1_i32)
            .set("min_radius", 20.0_f64)
            .set("max_radius", 600.0_f64)
            .set("center_tolerance", 14.0_f64)
            .set("min_arc_points", 24_usize)
            .set("min_ring_score", 50_u8)
            .set("colors", default_cross_colors()),
    );
    config.funcs_mut().insert(
        "debug_fun".to_string(),
        FuncConfig::new("debug_fun").set("message", "debug"),
    );
}

fn insert_sinks(config: &mut RuboConfig) {
    config.sinks_mut().insert(
        WEB_SINK_ID.to_string(),
        SinkConfig::new(WEB_SINK_ID).kind("web"),
    );
    config.sinks_mut().insert(
        UART_SINK_ID.to_string(),
        SinkConfig::new(UART_SINK_ID)
            .kind("uart")
            .set("serial", "/dev/ttyV0")
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
            .set("run_pin", 27_u8)
            .set("signals", default_gpio_signals()),
    );
}

fn insert_bindings(config: &mut RuboConfig) {
    insert_uart_binding(
        config,
        "uart_color_detect",
        "1",
        COLOR_CAMERA_ID,
        "color_detect",
    );
    insert_uart_binding(config, "uart_qr_detect", "2", QR_CAMERA_ID, "qr_detect");
    insert_uart_binding(config, "uart_cross_detect", "3", CROSS_CAMERA_ID, "cross");
    insert_uart_binding(
        config,
        "uart_black_ring_detect",
        "4",
        COLOR_CAMERA_ID,
        "black_ring_detect",
    );
    config.bindings_mut().insert(
        "debug".to_string(),
        BindingConfig::new("debug")
            .source("web", "debug")
            .func("debug_fun")
            .sink(WEB_SINK_ID)
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

fn default_color_ranges() -> Vec<serde_json::Value> {
    vec![
        color_range("red", [0, 50, 160, 255, 110, 255]),
        color_range("blue", [100, 137, 124, 255, 56, 255]),
        color_range("green", [50, 100, 91, 255, 85, 255]),
        color_range("black", [0, 179, 0, 255, 0, 76]),
        color_range("white", [0, 170, 0, 55, 120, 255]),
    ]
}

fn default_cross_colors() -> Vec<serde_json::Value> {
    vec![
        cross_color(1, "red", [0, 20, 100, 255, 80, 255]),
        cross_color(2, "blue", [90, 140, 80, 255, 50, 255]),
        cross_color(3, "green", [35, 90, 70, 255, 50, 255]),
        cross_color(4, "black", [0, 179, 0, 255, 0, 70]),
        cross_color(5, "white", [0, 179, 0, 60, 230, 255]),
    ]
}

fn default_target_correction() -> serde_json::Value {
    serde_json::json!({ "x": 0, "y": 0 })
}

fn color_range(name: &str, hsv: [i32; 6]) -> serde_json::Value {
    serde_json::json!({ "name": name, "hsv": hsv })
}

fn cross_color(id: u8, name: &str, hsv: [i32; 6]) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "name": name,
        "hsv": hsv,
        "min_area": 500.0,
        "min_circularity": 0.60
    })
}

fn default_gpio_signals() -> serde_json::Value {
    serde_json::json!({
        "color": 17,
        "qr": 22
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_rubo_config_test() {
        let config = default_rubo_config();
        assert_eq!(config.bindings().len(), 5);
        for id in [
            "uart_color_detect",
            "uart_qr_detect",
            "uart_cross_detect",
            "uart_black_ring_detect",
        ] {
            assert!(config.bindings()[id].debug_enabled());
        }
        let debug = &config.bindings()["debug"];
        assert_eq!(debug.source_ref().id(), "web");
        assert_eq!(debug.source_ref().event(), "debug");
        assert_eq!(debug.func_ref(), "debug_fun");
        assert_eq!(debug.sinks(), &["web".to_string()]);

        let mut engine = build_engine(".", default_app_config(), config);
        engine.prepare_web();
        assert_eq!(engine.config().bindings().len(), 5);
        let runtime_config = engine.web_state().unwrap().runtime_config();
        let runtime_config = runtime_config
            .read()
            .expect("test runtime config lock poisoned");
        assert!(runtime_config.validate());
        assert_eq!(runtime_config.bindings().len(), 9);
    }

    #[test]
    fn headless_config_test() {
        let app_config: AppConfig = serde_json::from_value(serde_json::json!({
            "web": { "enabled": false }
        }))
        .unwrap();
        let engine = build_engine(".", app_config, default_rubo_config());

        assert!(!engine.config().bindings().contains_key("debug"));
        assert!(engine.sinks().contains(WEB_SINK_ID));
    }
}
