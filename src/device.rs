#[cfg(feature = "opencv")]
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use rubo_engine::{
    Device, DeviceError,
    config::{ConfigAccess, DeviceConfig},
};

#[cfg(feature = "opencv")]
use opencv::{
    core::Mat,
    prelude::{MatTraitConst, VideoCaptureTrait, VideoCaptureTraitConst},
    videoio,
};

#[rubo_engine::device(kind = "camera")]
#[derive(Clone)]
pub struct CameraDevice {
    path: String,
    #[cfg(feature = "opencv")]
    capture: Arc<Mutex<videoio::VideoCapture>>,
}

impl CameraDevice {
    pub fn path(&self) -> &str {
        &self.path
    }

    #[cfg(feature = "opencv")]
    pub(crate) async fn frame(&self) -> Result<Mat, DeviceError> {
        let capture = self.capture.clone();
        tokio::task::spawn_blocking(move || {
            let mut capture = capture.lock().map_err(|_| DeviceError::Create {
                message: "camera lock poisoned".to_string(),
            })?;
            for _ in 0..30 {
                let mut frame = Mat::default();
                capture
                    .read(&mut frame)
                    .map_err(|error| DeviceError::Create {
                        message: error.to_string(),
                    })?;
                if !frame.empty() {
                    return Ok(frame);
                }
            }
            Err(DeviceError::Create {
                message: "camera returned empty frames".to_string(),
            })
        })
        .await
        .map_err(|error| DeviceError::Create {
            message: format!("camera read task failed: {error}"),
        })?
    }
}

#[async_trait]
impl Device for CameraDevice {
    async fn create(config: &DeviceConfig) -> Result<Self, DeviceError> {
        let path = config.get_or("path", "/dev/video0".to_string())?;
        #[cfg(feature = "opencv")]
        let capture = {
            let camera_path = path.clone();
            tokio::task::spawn_blocking(move || {
                let capture = videoio::VideoCapture::from_file(&camera_path, videoio::CAP_V4L2)
                    .map_err(|error| DeviceError::Create {
                        message: format!("failed to open camera {camera_path}: {error}"),
                    })?;
                if !capture.is_opened().map_err(|error| DeviceError::Create {
                    message: error.to_string(),
                })? {
                    return Err(DeviceError::Create {
                        message: format!("camera is not opened: {camera_path}"),
                    });
                }
                Ok(Arc::new(Mutex::new(capture)))
            })
            .await
            .map_err(|error| DeviceError::Create {
                message: format!("camera open task failed: {error}"),
            })??
        };
        Ok(Self {
            path,
            #[cfg(feature = "opencv")]
            capture,
        })
    }
}

#[cfg(all(test, feature = "opencv"))]
mod tests {
    #[tokio::test]
    #[ignore = "requires Ubuntu, OpenCV and a configured camera"]
    async fn camera_test() {
        let (_, camera) = crate::vision::test::load_camera("color_camera")
            .await
            .expect("load camera");
        let frame = camera.frame().await.expect("read camera frame");
        crate::vision::test::show_frame("camera_test", &frame).expect("show camera frame");
    }
}
