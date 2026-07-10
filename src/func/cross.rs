use async_trait::async_trait;
use rubo_engine::{FuncResult, Function, FunctionCall, FunctionError, config::ConfigAccess};

use crate::device::CameraDevice;

#[rubo_engine::function(id = "cross")]
#[derive(Default)]
pub struct CrossDetect;

#[async_trait]
impl Function for CrossDetect {
    async fn call(&self, call: FunctionCall<'_>) -> Result<FuncResult, FunctionError> {
        let device_id = call
            .function_config()
            .get_or("device_id", "camera".to_string())?;
        let runtime_param = call
            .message()
            .payload_ref()
            .get("runtime_param")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0) as u8;
        let camera = call.devices().get::<CameraDevice>(&device_id)?;
        run_cross_detect(camera, runtime_param, call.function_config()).await
    }
}

#[cfg(not(feature = "opencv"))]
async fn run_cross_detect(
    _camera: std::sync::Arc<CameraDevice>,
    _runtime_param: u8,
    _config: &rubo_engine::config::FuncConfig,
) -> Result<FuncResult, FunctionError> {
    Err(FunctionError::Call {
        message: crate::tool::opencv_disabled_message("cross"),
    })
}

#[cfg(feature = "opencv")]
async fn run_cross_detect(
    camera: std::sync::Arc<CameraDevice>,
    runtime_param: u8,
    config: &rubo_engine::config::FuncConfig,
) -> Result<FuncResult, FunctionError> {
    let output = cross_output(camera, runtime_param, config).await?;
    let image =
        crate::tool::mat_to_jpeg_data_url(&output.frame).map_err(|error| FunctionError::Call {
            message: error.to_string(),
        })?;
    Ok(FuncResult::new(serde_json::json!({
        "text": format!("cross finished: {}", output.value),
        "value": output.value,
        "image": image
    })))
}

#[cfg(feature = "opencv")]
async fn cross_output(
    camera: std::sync::Arc<CameraDevice>,
    runtime_param: u8,
    config: &rubo_engine::config::FuncConfig,
) -> Result<crate::vision::detect::VisionFrameOutput<String>, FunctionError> {
    let detect_config = crate::vision::detect::CrossDetectConfig {
        loop_count: config.get_or("loop_count", 1_i32)?,
        target_correction: config.get_or("target_correction", Default::default())?,
        black_threshold: config.get_or("black_threshold", 90_i32)?,
        close_kernel_size: config.get_or("close_kernel_size", 5_i32)?,
        dilate_kernel_size: config.get_or("dilate_kernel_size", 3_i32)?,
        dilate_iterations: config.get_or("dilate_iterations", 1_i32)?,
        min_radius: config.get_or("min_radius", 20.0_f64)?,
        max_radius: config.get_or("max_radius", 600.0_f64)?,
        center_tolerance: config.get_or("center_tolerance", 14.0_f64)?,
        min_arc_points: config.get_or("min_arc_points", 24_usize)?,
        min_ring_score: config.get_or("min_ring_score", 50_u8)?,
        colors: config.get("colors")?,
    };
    let frames = super::read_frames(&camera, detect_config.loop_count).await?;
    tokio::task::spawn_blocking(move || {
        crate::vision::detect::detect_cross(frames, &detect_config, runtime_param)
    })
    .await
    .map_err(|error| FunctionError::Call {
        message: format!("cross task failed: {error}"),
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
    async fn cross_test() {
        let (config, camera) = crate::vision::test::load_camera("camera")
            .await
            .expect("load cross camera");
        let output = cross_output(camera, 0, &config.funcs()["cross"])
            .await
            .expect("detect cross");
        crate::vision::test::show_frame("cross_test", &output.frame).expect("show frame");
    }
}
