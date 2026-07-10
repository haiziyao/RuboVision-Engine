use async_trait::async_trait;
use rubo_engine::{FuncResult, Function, FunctionCall, FunctionError, config::ConfigAccess};

use crate::device::CameraDevice;

#[rubo_engine::function(id = "color_detect")]
#[derive(Default)]
pub struct ColorDetect;

#[async_trait]
impl Function for ColorDetect {
    async fn call(&self, call: FunctionCall<'_>) -> Result<FuncResult, FunctionError> {
        let device_id = call
            .function_config()
            .get_or("device_id", "color_camera".to_string())?;
        let camera = call.devices().get::<CameraDevice>(&device_id)?;
        run_color_detect(camera.path(), call.function_config()).await
    }
}

#[cfg(not(feature = "opencv"))]
async fn run_color_detect(
    _camera_path: &str,
    _config: &rubo_engine::config::FuncConfig,
) -> Result<FuncResult, FunctionError> {
    Err(FunctionError::Call {
        message: crate::tool::opencv_disabled_message("color_detect"),
    })
}

#[cfg(feature = "opencv")]
async fn run_color_detect(
    camera_path: &str,
    config: &rubo_engine::config::FuncConfig,
) -> Result<FuncResult, FunctionError> {
    let detect_config = crate::vision::detect::ColorDetectConfig {
        path: camera_path.to_string(),
        loop_count: config.get_or("loop_count", 1_i32)?,
        radius_ratio: config.get_or("radius_ratio", 0.4_f64)?,
        detect_area_access_rate: config.get_or("detect_area_access_rate", 0.8_f64)?,
        color_ranges: config.get("color_ranges")?,
    };
    let output = crate::vision::detect::detect_color(&detect_config).map_err(|error| {
        FunctionError::Call {
            message: error.to_string(),
        }
    })?;
    let image =
        crate::tool::mat_to_jpeg_data_url(&output.frame).map_err(|error| FunctionError::Call {
            message: error.to_string(),
        })?;
    Ok(FuncResult::new(serde_json::json!({
        "text": format!("color_detect finished: {}", output.value),
        "value": output.value,
        "image": image
    })))
}

#[cfg(all(test, feature = "opencv"))]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires OpenCV runtime and camera/image environment"]
    fn color_detect_test() {
        let config = crate::vision::detect::ColorDetectConfig {
            path: "/dev/video2".to_string(),
            loop_count: 5,
            radius_ratio: 0.4,
            detect_area_access_rate: 0.8,
            color_ranges: vec![
                crate::vision::detect::ColorRange {
                    name: "red".to_string(),
                    hsv: [0, 50, 160, 255, 110, 255],
                },
                crate::vision::detect::ColorRange {
                    name: "blue".to_string(),
                    hsv: [100, 137, 124, 255, 56, 255],
                },
                crate::vision::detect::ColorRange {
                    name: "green".to_string(),
                    hsv: [50, 100, 91, 255, 85, 255],
                },
            ],
        };
        let output = crate::vision::detect::detect_color(&config).expect("detect color");
        crate::vision::test::show_frame("color_detect_test", &output.frame).expect("show frame");
    }
}
