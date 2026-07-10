use rubo_engine_macros::{device, function, sink, source};

#[source(kind = "timer")]
fn timer_source_marker() -> &'static str {
    "source"
}

#[device(kind = "camera")]
fn camera_device_marker() -> &'static str {
    "device"
}

#[function(id = "detect")]
fn detect_function_marker() -> &'static str {
    "function"
}

#[sink(id = "console")]
mod console_sink {
    pub fn enabled() -> bool {
        true
    }
}

#[test]
fn attribute_macros_preserve_annotated_items() {
    assert_eq!(timer_source_marker(), "source");
    assert_eq!(camera_device_marker(), "device");
    assert_eq!(detect_function_marker(), "function");
    assert!(console_sink::enabled());
}
