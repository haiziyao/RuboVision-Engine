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
            .get_or("device_id", "camera".to_string())?;
        let camera = call.devices().get::<CameraDevice>(&device_id)?;
        run_color_detect(camera, call.function_config(), call.image_enabled()).await
    }
}

#[cfg(not(feature = "opencv"))]
async fn run_color_detect(
    _camera: std::sync::Arc<CameraDevice>,
    _config: &rubo_engine::config::FuncConfig,
    _image_enabled: bool,
) -> Result<FuncResult, FunctionError> {
    Err(FunctionError::Call {
        message: crate::tool::opencv_disabled_message("color_detect"),
    })
}

#[cfg(feature = "opencv")]
async fn run_color_detect(
    camera: std::sync::Arc<CameraDevice>,
    config: &rubo_engine::config::FuncConfig,
    image_enabled: bool,
) -> Result<FuncResult, FunctionError> {
    let output = color_detect_output(camera, config).await?;
    let mut result = serde_json::json!({
        "text": format!("color_detect finished: {}", output.value),
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
async fn color_detect_output(
    camera: std::sync::Arc<CameraDevice>,
    config: &rubo_engine::config::FuncConfig,
) -> Result<crate::vision::detect::VisionFrameOutput<String>, FunctionError> {
    let detect_config = crate::vision::detect::ColorDetectConfig {
        loop_count: config.get_or("loop_count", 1_i32)?,
        radius_ratio: config.get_or("radius_ratio", 0.4_f64)?,
        detect_area_access_rate: config.get_or("detect_area_access_rate", 0.8_f64)?,
        color_ranges: config.get("color_ranges")?,
    };
    let frames = super::read_frames(&camera, detect_config.loop_count).await?;
    tokio::task::spawn_blocking(move || crate::vision::detect::detect_color(frames, &detect_config))
        .await
        .map_err(|error| FunctionError::Call {
            message: format!("color_detect task failed: {error}"),
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
    async fn test_color() {
        let (config, camera) = crate::vision::test::load_camera("camera")
            .await
            .expect("load color camera");
        let mut devices = FunctionDevices::new();
        devices.insert("camera", &camera);
        let message = Message::new("test_color");
        let call = FunctionCall::new(&config.funcs()["color_detect"], &message, devices, true);
        let result = ColorDetect.call(call).await.expect("run color function");
        println!("color value={}", result.value()["value"]);
    }

    #[tokio::test]
    async fn test_color_show() {
        let (config, camera) = crate::vision::test::load_camera("camera")
            .await
            .expect("load color camera");
        let camera = camera.get::<CameraDevice>().expect("get color camera");
        loop {
            let output = color_detect_output(camera.clone(), &config.funcs()["color_detect"])
                .await
                .expect("detect color");
            println!("color value={}", output.value);
            highgui::imshow("test_color_show", &output.frame).expect("show color frame");
            let key = highgui::wait_key(1).expect("wait for color key") & 0xff;
            if key == 113 || key == 27 {
                break;
            }
        }
    }
}
