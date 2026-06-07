mod camera;
mod color;
mod config;
mod cross;
mod qr;

#[cfg(test)]
mod tests;

pub use color::run_color_detect;
pub use config::{CameraDevice, ColorDetectConfig, CrossDetectConfig, QrDetectConfig};
pub use cross::run_cross_detect;
pub use qr::run_qr_detect;
