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
        run_qr_detect(camera, call.function_config()).await
    }
}

#[cfg(not(feature = "opencv"))]
async fn run_qr_detect(
    _camera: std::sync::Arc<CameraDevice>,
    _config: &rubo_engine::config::FuncConfig,
) -> Result<FuncResult, FunctionError> {
    Err(FunctionError::Call {
        message: crate::tool::opencv_disabled_message("qr_detect"),
    })
}

#[cfg(feature = "opencv")]
async fn run_qr_detect(
    camera: std::sync::Arc<CameraDevice>,
    config: &rubo_engine::config::FuncConfig,
) -> Result<FuncResult, FunctionError> {
    let output = qr_detect_output(camera, config).await?;
    if output.value.is_empty() {
        return Err(FunctionError::Call {
            message: "qr code not found before loop_count was reached".to_string(),
        });
    }
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

#[cfg(feature = "opencv")]
async fn qr_detect_output(
    camera: std::sync::Arc<CameraDevice>,
    config: &rubo_engine::config::FuncConfig,
) -> Result<crate::vision::detect::VisionFrameOutput<String>, FunctionError> {
    let loop_count = config.get_or("loop_count", 30_i32)?;
    let frames = super::read_frames(&camera, loop_count).await?;
    tokio::task::spawn_blocking(move || crate::vision::detect::detect_qr(frames))
        .await
        .map_err(|error| FunctionError::Call {
            message: format!("qr_detect task failed: {error}"),
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
    async fn qr_detect_test() {
        let (config, camera) = crate::vision::test::load_camera("qr_camera")
            .await
            .expect("load qr camera");
        let output = qr_detect_output(camera, &config.funcs()["qr_detect"])
            .await
            .expect("detect qr");
        crate::vision::test::show_frame("qr_detect_test", &output.frame).expect("show frame");
    }
}
