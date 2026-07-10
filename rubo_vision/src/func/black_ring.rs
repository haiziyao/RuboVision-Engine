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
        run_black_ring_detect(camera.path(), call.function_config()).await
    }
}

#[cfg(not(feature = "opencv"))]
async fn run_black_ring_detect(
    _camera_path: &str,
    _config: &rubo_engine::config::FuncConfig,
) -> Result<FuncResult, FunctionError> {
    Err(FunctionError::Call {
        message: crate::tool::opencv_disabled_message("black_ring_detect"),
    })
}

#[cfg(feature = "opencv")]
async fn run_black_ring_detect(
    camera_path: &str,
    config: &rubo_engine::config::FuncConfig,
) -> Result<FuncResult, FunctionError> {
    let detect_config = crate::vision::detect::BlackRingDetectConfig {
        path: camera_path.to_string(),
        loop_count: config.get_or("loop_count", 1_i32)?,
        target_correction: config.get_or("target_correction", Default::default())?,
        black_threshold: config.get_or("black_threshold", 90_i32)?,
        min_radius: config.get_or("min_radius", 20.0_f64)?,
        max_radius: config.get_or("max_radius", 180.0_f64)?,
        min_circularity: config.get_or("min_circularity", 0.65_f64)?,
        min_score: config.get_or("min_score", 50_u8)?,
    };
    let output = crate::vision::detect::detect_black_ring(&detect_config).map_err(|error| {
        FunctionError::Call {
            message: error.to_string(),
        }
    })?;
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

#[cfg(all(test, feature = "opencv"))]
mod tests {
    #[test]
    #[ignore = "requires OpenCV runtime and camera/image environment"]
    fn black_ring_detect_test() {
        let config = crate::vision::detect::BlackRingDetectConfig {
            path: "/dev/video2".to_string(),
            loop_count: 3,
            target_correction: Default::default(),
            black_threshold: 90,
            min_radius: 20.0,
            max_radius: 180.0,
            min_circularity: 0.65,
            min_score: 50,
        };
        let output = crate::vision::detect::detect_black_ring(&config).expect("detect black ring");
        crate::vision::test::show_frame("black_ring_detect_test", &output.frame)
            .expect("show frame");
    }
}
