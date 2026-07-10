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
            .get_or("device_id", "cross_camera".to_string())?;
        let runtime_param = call
            .message()
            .payload_ref()
            .get("runtime_param")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0) as u8;
        let camera = call.devices().get::<CameraDevice>(&device_id)?;
        run_cross_detect(camera.path(), runtime_param, call.function_config()).await
    }
}

#[cfg(not(feature = "opencv"))]
async fn run_cross_detect(
    _camera_path: &str,
    _runtime_param: u8,
    _config: &rubo_engine::config::FuncConfig,
) -> Result<FuncResult, FunctionError> {
    Err(FunctionError::Call {
        message: crate::tool::opencv_disabled_message("cross"),
    })
}

#[cfg(feature = "opencv")]
async fn run_cross_detect(
    camera_path: &str,
    runtime_param: u8,
    config: &rubo_engine::config::FuncConfig,
) -> Result<FuncResult, FunctionError> {
    let detect_config = crate::vision::detect::CrossDetectConfig {
        path: camera_path.to_string(),
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
    let output =
        crate::vision::detect::detect_cross(&detect_config, runtime_param).map_err(|error| {
            FunctionError::Call {
                message: error.to_string(),
            }
        })?;
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

#[cfg(all(test, feature = "opencv"))]
mod tests {
    #[test]
    #[ignore = "requires OpenCV runtime and camera/image environment"]
    fn cross_test() {
        let config = crate::vision::detect::CrossDetectConfig {
            path: "/dev/video2".to_string(),
            loop_count: 3,
            target_correction: Default::default(),
            black_threshold: 90,
            close_kernel_size: 5,
            dilate_kernel_size: 3,
            dilate_iterations: 1,
            min_radius: 20.0,
            max_radius: 600.0,
            center_tolerance: 14.0,
            min_arc_points: 24,
            min_ring_score: 50,
            colors: vec![
                crate::vision::detect::CrossColor {
                    id: 1,
                    name: "red".to_string(),
                    hsv: [0, 20, 100, 255, 80, 255],
                    min_area: 500.0,
                    min_circularity: 0.60,
                },
                crate::vision::detect::CrossColor {
                    id: 2,
                    name: "blue".to_string(),
                    hsv: [90, 140, 80, 255, 50, 255],
                    min_area: 500.0,
                    min_circularity: 0.60,
                },
                crate::vision::detect::CrossColor {
                    id: 3,
                    name: "green".to_string(),
                    hsv: [35, 90, 70, 255, 50, 255],
                    min_area: 500.0,
                    min_circularity: 0.60,
                },
            ],
        };
        let output = crate::vision::detect::detect_cross(&config, 0).expect("detect cross");
        crate::vision::test::show_frame("cross_test", &output.frame).expect("show frame");
    }
}
