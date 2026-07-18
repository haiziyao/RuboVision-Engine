use async_trait::async_trait;
use rubo_engine::{FuncResult, Function, FunctionCall, FunctionError, config::ConfigAccess};

use crate::device::CameraDevice;

#[rubo_engine::function(id = "black_ring_detect")]
#[derive(Default)]
pub struct BlackRingDetect;

#[async_trait]
impl Function for BlackRingDetect {
    async fn call(&self, call: FunctionCall<'_>) -> Result<FuncResult, FunctionError> {
        let device_id = call
            .function_config()
            .get_or("device_id", "camera".to_string())?;
        let camera = call.devices().get::<CameraDevice>(&device_id)?;
        run_black_ring_detect(camera, call.function_config(), call.image_enabled()).await
    }
}

#[cfg(not(feature = "opencv"))]
async fn run_black_ring_detect(
    _camera: std::sync::Arc<CameraDevice>,
    _config: &rubo_engine::config::FuncConfig,
    _image_enabled: bool,
) -> Result<FuncResult, FunctionError> {
    Err(FunctionError::Call {
        message: crate::tool::opencv_disabled_message("black_ring_detect"),
    })
}

#[cfg(feature = "opencv")]
async fn run_black_ring_detect(
    camera: std::sync::Arc<CameraDevice>,
    config: &rubo_engine::config::FuncConfig,
    image_enabled: bool,
) -> Result<FuncResult, FunctionError> {
    let output = black_ring_detect_output(camera, config).await?;
    let mut result = serde_json::json!({
        "text": format!("black_ring_detect finished: {}", output.value),
        "value": output.value
    });
    if image_enabled {
        result["image"] = crate::tool::mat_to_jpeg_data_url(&output.frame)
            .map(serde_json::Value::String)
            .map_err(|error| FunctionError::Call {
                message: error.to_string(),
            })?;
    }
    Ok(FuncResult::new(result))
}

#[cfg(feature = "opencv")]
async fn black_ring_detect_output(
    camera: std::sync::Arc<CameraDevice>,
    config: &rubo_engine::config::FuncConfig,
) -> Result<crate::vision::detect::VisionFrameOutput<String>, FunctionError> {
    let detect_config = crate::vision::detect::BlackRingDetectConfig {
        loop_count: config.get_or("loop_count", 1_i32)?,
        target_correction: config.get_or("target_correction", Default::default())?,
        black_threshold: config.get_or("black_threshold", 90_i32)?,
        min_radius: config.get_or("min_radius", 20.0_f64)?,
        max_radius: config.get_or("max_radius", 180.0_f64)?,
        min_circularity: config.get_or("min_circularity", 0.65_f64)?,
        min_score: config.get_or("min_score", 50_u8)?,
    };
    let frames = super::read_frames(&camera, detect_config.loop_count).await?;
    tokio::task::spawn_blocking(move || {
        crate::vision::detect::detect_black_ring(frames, &detect_config)
    })
    .await
    .map_err(|error| FunctionError::Call {
        message: format!("black_ring_detect task failed: {error}"),
    })?
    .map_err(|error| FunctionError::Call {
        message: error.to_string(),
    })
}

#[cfg(all(test, feature = "opencv"))]
mod tests {
    use opencv::highgui;
    use rubo_engine::{FunctionDevices, Message};

    use super::*;

    #[tokio::test]
    async fn test_black_ring() {
        let (config, camera) = crate::vision::test::load_camera("camera")
            .await
            .expect("load black ring camera");
        let mut devices = FunctionDevices::new();
        devices.insert("camera", &camera);
        let message = Message::new("test_black_ring");
        let call = FunctionCall::new(
            &config.funcs()["black_ring_detect"],
            &message,
            devices,
            true,
        );
        let result = BlackRingDetect
            .call(call)
            .await
            .expect("run black ring function");
        println!("black ring value={}", result.value()["value"]);
    }

    #[tokio::test]
    async fn test_black_ring_show() {
        let (config, camera) = crate::vision::test::load_camera("camera")
            .await
            .expect("load black ring camera");
        let camera = camera.get::<CameraDevice>().expect("get black ring camera");
        loop {
            let output =
                black_ring_detect_output(camera.clone(), &config.funcs()["black_ring_detect"])
                    .await
                    .expect("detect black ring");
            println!("black ring value={}", output.value);
            highgui::imshow("test_black_ring_show", &output.frame).expect("show black ring frame");
            let key = highgui::wait_key(1).expect("wait for black ring key") & 0xff;
            if key == 113 || key == 27 {
                break;
            }
        }
    }
}
