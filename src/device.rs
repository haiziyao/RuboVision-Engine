use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

#[cfg(feature = "opencv")]
use std::sync::Mutex;

use async_trait::async_trait;
use rubo_engine::{
    Device, DeviceError, FunctionAspect, Output, Sink, SinkError, TaskRequest,
    config::{ConfigAccess, DeviceConfig, SinkConfig},
};

#[cfg(feature = "opencv")]
use opencv::{
    core::Mat,
    prelude::{MatTraitConst, VideoCaptureTrait, VideoCaptureTraitConst},
    videoio,
};

#[rubo_engine::device(kind = "camera")]
#[derive(Clone)]
pub struct CameraDevice {
    path: String,
    #[cfg(feature = "opencv")]
    capture: Arc<Mutex<videoio::VideoCapture>>,
}

impl CameraDevice {
    pub fn path(&self) -> &str {
        &self.path
    }

    #[cfg(feature = "opencv")]
    pub(crate) async fn frame(&self) -> Result<Mat, DeviceError> {
        let capture = self.capture.clone();
        tokio::task::spawn_blocking(move || {
            let mut capture = capture.lock().map_err(|_| DeviceError::Create {
                message: "camera lock poisoned".to_string(),
            })?;
            for _ in 0..30 {
                let mut frame = Mat::default();
                capture
                    .read(&mut frame)
                    .map_err(|error| DeviceError::Create {
                        message: error.to_string(),
                    })?;
                if !frame.empty() {
                    return Ok(frame);
                }
            }
            Err(DeviceError::Create {
                message: "camera returned empty frames".to_string(),
            })
        })
        .await
        .map_err(|error| DeviceError::Create {
            message: format!("camera read task failed: {error}"),
        })?
    }
}

#[async_trait]
impl Device for CameraDevice {
    async fn create(config: &DeviceConfig) -> Result<Self, DeviceError> {
        let path = config.get_or("path", "/dev/video0".to_string())?;
        #[cfg(feature = "opencv")]
        let capture = {
            let camera_path = path.clone();
            tokio::task::spawn_blocking(move || {
                let capture = videoio::VideoCapture::from_file(&camera_path, videoio::CAP_V4L2)
                    .map_err(|error| DeviceError::Create {
                        message: format!("failed to open camera {camera_path}: {error}"),
                    })?;
                if !capture.is_opened().map_err(|error| DeviceError::Create {
                    message: error.to_string(),
                })? {
                    return Err(DeviceError::Create {
                        message: format!("camera is not opened: {camera_path}"),
                    });
                }
                Ok(Arc::new(Mutex::new(capture)))
            })
            .await
            .map_err(|error| DeviceError::Create {
                message: format!("camera open task failed: {error}"),
            })??
        };
        Ok(Self {
            path,
            #[cfg(feature = "opencv")]
            capture,
        })
    }
}

#[derive(Clone)]
pub struct GpioDevice {
    state: Arc<GpioState>,
}

impl GpioDevice {
    pub fn from_config(config: &SinkConfig) -> Result<Self, SinkError> {
        let active_low = config.get_or("active_low", true)?;
        let chip = config.get_or("chip", 0_u8)?;
        let run_pin = config.get::<u32>("run_pin")?;
        let signals = config.get_or::<serde_json::Value>("signals", serde_json::json!({}))?;
        let mut pins = vec![run_pin];
        if let Some(signals) = signals.as_object() {
            pins.extend(
                signals
                    .values()
                    .filter_map(serde_json::Value::as_u64)
                    .filter_map(|pin| u32::try_from(pin).ok()),
            );
        }
        pins.sort_unstable();
        pins.dedup();
        Ok(Self {
            state: Arc::new(GpioState::new(chip, active_low, pins)),
        })
    }

    fn begin(&self) {
        if self.state.active_count.fetch_add(1, Ordering::SeqCst) == 0 {
            self.state.set_active(true);
        }
    }

    fn end(&self) {
        let previous = self
            .state
            .active_count
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |count| {
                count.checked_sub(1)
            })
            .unwrap_or(0);
        if previous == 1 {
            self.state.set_active(false);
        }
    }
}

impl Default for GpioDevice {
    fn default() -> Self {
        Self {
            state: Arc::new(GpioState::new(0, true, Vec::new())),
        }
    }
}

#[async_trait]
impl FunctionAspect for GpioDevice {
    async fn before(&self, _task: &TaskRequest) {
        self.begin();
    }

    async fn after(&self, _output: &Output) {
        self.end();
    }
}

#[async_trait]
impl Sink for GpioDevice {
    async fn handle(&self, _output: &Output, _config: &SinkConfig) -> Result<(), SinkError> {
        Ok(())
    }
}

#[cfg_attr(not(all(feature = "hardware", target_os = "linux")), allow(dead_code))]
struct GpioState {
    chip: u8,
    active_low: bool,
    pins: Vec<u32>,
    active: AtomicBool,
    active_count: AtomicUsize,
    #[cfg(all(feature = "hardware", target_os = "linux"))]
    output_lines: std::sync::Mutex<Option<gpiod::Lines<gpiod::Output>>>,
}

impl GpioState {
    fn new(chip: u8, active_low: bool, pins: Vec<u32>) -> Self {
        Self {
            chip,
            active_low,
            pins,
            active: AtomicBool::new(false),
            active_count: AtomicUsize::new(0),
            #[cfg(all(feature = "hardware", target_os = "linux"))]
            output_lines: std::sync::Mutex::new(None),
        }
    }

    fn set_active(&self, active: bool) {
        self.active.store(active, Ordering::SeqCst);
        #[cfg(all(feature = "hardware", target_os = "linux"))]
        if let Err(error) = self.write_pins(active) {
            eprintln!("gpio write failed: {error}");
        }
    }

    #[cfg(all(feature = "hardware", target_os = "linux"))]
    fn write_pins(&self, active: bool) -> Result<(), String> {
        if self.pins.is_empty() {
            return Ok(());
        }
        let mut output_lines = self
            .output_lines
            .lock()
            .map_err(|_| "gpio output lock poisoned".to_string())?;
        let high = active != self.active_low;
        if output_lines.is_none() {
            let chip_path = format!("/dev/gpiochip{}", self.chip);
            let chip = gpiod::Chip::new(chip_path.as_str()).map_err(|error| error.to_string())?;
            let options = gpiod::Options::output(self.pins.clone())
                .values(vec![high; self.pins.len()])
                .consumer("rubo_vision");
            *output_lines = Some(
                chip.request_lines(options)
                    .map_err(|error| error.to_string())?,
            );
        }
        output_lines
            .as_ref()
            .expect("gpio lines initialized")
            .set_values(vec![high; self.pins.len()])
            .map_err(|error| error.to_string())
    }
}

impl Drop for GpioState {
    fn drop(&mut self) {
        if self.active.load(Ordering::SeqCst) {
            self.set_active(false);
        }
    }
}

#[cfg(all(test, feature = "opencv"))]
mod tests {
    #[tokio::test]
    #[ignore = "requires Ubuntu, OpenCV and a configured camera"]
    async fn camera_test() {
        let (_, camera) = crate::vision::test::load_camera("camera")
            .await
            .expect("load camera");
        let camera = camera.get::<super::CameraDevice>().expect("get camera");
        let frame = camera.frame().await.expect("read camera frame");
        crate::vision::test::show_frame("camera_test", &frame).expect("show camera frame");
    }
}

#[cfg(test)]
mod gpio_tests {
    use std::sync::atomic::Ordering;

    use rubo_engine::config::{ConfigAccess, SinkConfig};

    #[test]
    fn gpio_device_test() {
        let config = SinkConfig::new("gpio")
            .kind("gpio")
            .set("chip", 0_u8)
            .set("run_pin", 27_u32)
            .set("active_low", true);
        let gpio = super::GpioDevice::from_config(&config).unwrap();

        gpio.begin();
        gpio.begin();
        assert!(gpio.state.active.load(Ordering::SeqCst));
        assert_eq!(gpio.state.active_count.load(Ordering::SeqCst), 2);

        gpio.end();
        assert!(gpio.state.active.load(Ordering::SeqCst));
        gpio.end();
        assert!(!gpio.state.active.load(Ordering::SeqCst));
        assert_eq!(gpio.state.active_count.load(Ordering::SeqCst), 0);
    }
}
