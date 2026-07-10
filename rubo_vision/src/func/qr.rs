use async_trait::async_trait;
use rubo_engine::{FuncResult, Function, FunctionCall, FunctionError, config::ConfigAccess};

use crate::device::CameraDevice;

#[rubo_engine::function(id = "qr_detect")]
#[derive(Default)]
pub struct QrDetect;

#[async_trait]
impl Function for QrDetect {
    async fn call(&self, call: FunctionCall<'_>) -> Result<FuncResult, FunctionError> {
        let device_id = call
            .function_config()
            .get_or("device_id", "qr_camera".to_string())?;
        let camera = call.devices().get::<CameraDevice>(&device_id)?;
        run_qr_detect(camera.path()).await
    }
}

#[cfg(not(feature = "opencv"))]
async fn run_qr_detect(_camera_path: &str) -> Result<FuncResult, FunctionError> {
    Err(FunctionError::Call {
        message: crate::tool::opencv_disabled_message("qr_detect"),
    })
}

#[cfg(feature = "opencv")]
async fn run_qr_detect(camera_path: &str) -> Result<FuncResult, FunctionError> {
    let output =
        crate::vision::detect::detect_qr(camera_path).map_err(|error| FunctionError::Call {
            message: error.to_string(),
        })?;
    let image =
        crate::tool::mat_to_jpeg_data_url(&output.frame).map_err(|error| FunctionError::Call {
            message: error.to_string(),
        })?;
    Ok(FuncResult::new(serde_json::json!({
        "text": format!("qr_detect finished: {}", output.value),
        "value": output.value,
        "image": image
    })))
}

#[cfg(all(test, feature = "opencv"))]
mod tests {
    #[test]
    #[ignore = "requires OpenCV runtime and camera/image environment"]
    fn qr_detect_test() {
        let output = crate::vision::detect::detect_qr("/dev/video2").expect("detect qr");
        crate::vision::test::show_frame("qr_detect_test", &output.frame).expect("show frame");
    }
}
