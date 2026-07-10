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
            .get_or("device_id", "color_camera".to_string())?;
        let camera = call.devices().get::<CameraDevice>(&device_id)?;
        run_black_ring_detect(camera, call.function_config()).await
    }
}

#[cfg(not(feature = "opencv"))]
async fn run_black_ring_detect(
    _camera: std::sync::Arc<CameraDevice>,
    _config: &rubo_engine::config::FuncConfig,
) -> Result<FuncResult, FunctionError> {
    Err(FunctionError::Call {
        message: crate::tool::opencv_disabled_message("black_ring_detect"),
    })
}

#[cfg(feature = "opencv")]
async fn run_black_ring_detect(
    camera: std::sync::Arc<CameraDevice>,
    config: &rubo_engine::config::FuncConfig,
) -> Result<FuncResult, FunctionError> {
    let output = black_ring_detect_output(camera, config).await?;
    let image =
        crate::tool::mat_to_jpeg_data_url(&output.frame).map_err(|error| FunctionError::Call {
            message: error.to_string(),
        })?;
    Ok(FuncResult::new(serde_json::json!({
        "text": format!("black_ring_detect finished: {}", output.value),
        "value": output.value,
        "image": image
    })))
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
    use super::*;

    #[tokio::test]
    #[ignore = "requires Ubuntu, OpenCV and a configured camera"]
    async fn black_ring_detect_test() {
        let (config, camera) = crate::vision::test::load_camera("color_camera")
            .await
            .expect("load color camera");
        let output = black_ring_detect_output(camera, &config.funcs()["black_ring_detect"])
            .await
            .expect("detect black ring");
        crate::vision::test::show_frame("black_ring_detect_test", &output.frame)
            .expect("show frame");
    }
}
