use async_trait::async_trait;
use rubo_engine::{
    FuncResult, Function, FunctionCall, FunctionError,
    config::{ConfigAccess, FuncConfig},
};

use crate::device::CameraDevice;

#[derive(Debug, Clone)]
struct QrParameters {
    device_id: String,
    max_frames: usize,
}

impl QrParameters {
    fn from_config(config: &FuncConfig) -> Result<Self, FunctionError> {
        let parameters = Self {
            device_id: config.get("device_id")?,
            max_frames: config.get("max_frames")?,
        };
        if parameters.device_id.trim().is_empty() {
            return Err(FunctionError::Config {
                message: "qr_detect.device_id cannot be empty".to_string(),
            });
        }
        if parameters.max_frames == 0 {
            return Err(FunctionError::Config {
                message: "qr_detect.max_frames must be greater than 0".to_string(),
            });
        }
        Ok(parameters)
    }
}

#[cfg(feature = "opencv")]
struct QrFrameResult {
    value: Option<String>,
    gray: opencv::core::Mat,
    frame: opencv::core::Mat,
}

#[cfg(feature = "opencv")]
struct QrResult {
    value: String,
    frame: opencv::core::Mat,
}

#[cfg(feature = "opencv")]
fn analyze_qr_frame(frame: &opencv::core::Mat) -> opencv::Result<QrFrameResult> {
    use opencv::prelude::*;

    let gray = crate::vision::util::bgr_to_gray(frame)?;
    let value = decode_qr(&gray)?;
    Ok(QrFrameResult {
        value,
        gray,
        frame: frame.try_clone()?,
    })
}

#[cfg(feature = "opencv")]
fn decode_qr(gray: &opencv::core::Mat) -> opencv::Result<Option<String>> {
    use opencv::prelude::*;

    let size = gray.size()?;
    let data = gray.data_bytes()?;
    let mut decoder = quircs::Quirc::default();
    for code in decoder
        .identify(size.width as usize, size.height as usize, data)
        .flatten()
    {
        let Ok(decoded) = code.decode() else {
            continue;
        };
        let Ok(text) = std::str::from_utf8(&decoded.payload) else {
            continue;
        };
        if !text.is_empty() {
            return Ok(Some(text.to_string()));
        }
    }
    Ok(None)
}

#[cfg(feature = "opencv")]
async fn detect_qr(
    camera: std::sync::Arc<CameraDevice>,
    parameters: &QrParameters,
) -> Result<QrResult, FunctionError> {
    for _ in 0..parameters.max_frames {
        let frame = camera.frame().await?;
        let result = tokio::task::spawn_blocking(move || analyze_qr_frame(&frame))
            .await
            .map_err(|error| FunctionError::Call {
                message: format!("qr_detect task failed: {error}"),
            })?
            .map_err(|error| FunctionError::Call {
                message: error.to_string(),
            })?;
        if let Some(value) = result.value {
            return Ok(QrResult {
                value,
                frame: result.frame,
            });
        }
    }
    Err(FunctionError::Call {
        message: format!(
            "qr code was not detected within {} frames",
            parameters.max_frames
        ),
    })
}

#[rubo_engine::function(id = "qr_detect")]
#[derive(Default)]
pub struct QrDetect;

#[async_trait]
impl Function for QrDetect {
    async fn call(&self, call: FunctionCall<'_>) -> Result<FuncResult, FunctionError> {
        let parameters = QrParameters::from_config(call.function_config())?;
        let camera = call.devices().get::<CameraDevice>(&parameters.device_id)?;
        run_qr_detect(camera, &parameters, call.image_enabled()).await
    }
}

#[cfg(not(feature = "opencv"))]
async fn run_qr_detect(
    _camera: std::sync::Arc<CameraDevice>,
    _parameters: &QrParameters,
    _image_enabled: bool,
) -> Result<FuncResult, FunctionError> {
    Err(FunctionError::Call {
        message: crate::tool::opencv_disabled_message("qr_detect"),
    })
}

#[cfg(feature = "opencv")]
async fn run_qr_detect(
    camera: std::sync::Arc<CameraDevice>,
    parameters: &QrParameters,
    image_enabled: bool,
) -> Result<FuncResult, FunctionError> {
    let result = detect_qr(camera, parameters).await?;
    let mut output = serde_json::json!({
        "text": format!("qr_detect finished: {}", result.value),
        "value": result.value
    });
    if image_enabled {
        output["image"] = crate::tool::mat_to_jpeg_data_url(&result.frame)
            .map(serde_json::Value::String)
            .map_err(|error| FunctionError::Call {
                message: error.to_string(),
            })?;
    }
    Ok(FuncResult::new(output))
}

#[cfg(all(test, feature = "opencv"))]
mod tests {
    use opencv::{highgui, prelude::*};

    use super::*;

    #[tokio::test]
    async fn test_qr() {
        let (config, camera) = crate::vision::test::load_camera("task")
            .await
            .expect("load qr camera");
        let camera = camera.get::<CameraDevice>().expect("get qr camera");
        let parameters =
            QrParameters::from_config(&config.funcs()["qr_detect"]).expect("load qr config");
        let result = detect_qr(camera, &parameters).await.expect("detect qr");
        println!("qr={}", result.value);
    }

    #[tokio::test]
    async fn test_qr_show() {
        let (config, camera) = crate::vision::test::load_camera("task")
            .await
            .expect("load qr camera");
        let camera = camera.get::<CameraDevice>().expect("get qr camera");
        QrParameters::from_config(&config.funcs()["qr_detect"]).expect("load qr config");
        loop {
            let frame = camera.frame().await.expect("read qr frame");
            let frame_for_analysis = frame.try_clone().expect("clone qr frame");
            let result = tokio::task::spawn_blocking(move || analyze_qr_frame(&frame_for_analysis))
                .await
                .expect("join qr analysis")
                .expect("analyze qr frame");
            if let Some(value) = result.value.as_deref() {
                println!("qr={value}");
            }
            let display = crate::vision::test::annotate_result(
                &result.frame,
                "QR",
                result.value.as_deref().unwrap_or(""),
            )
            .expect("annotate qr result");
            highgui::imshow("qr.original", &display).expect("show qr original");
            highgui::imshow("qr.gray", &result.gray).expect("show qr gray");
            let key = highgui::wait_key(1).expect("wait for qr key") & 0xff;
            if key == 113 || key == 27 {
                break;
            }
        }
    }
}
