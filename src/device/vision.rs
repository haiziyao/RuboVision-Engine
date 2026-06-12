mod black_ring;
mod camera;
mod color;
mod config;
mod cross;
mod qr;

#[cfg(test)]
mod tests;

pub use black_ring::{
    BlackRingDetectOutput, format_black_ring_value, run_black_ring_detect_with_frame,
};
#[cfg(test)]
pub use black_ring::{BlackRingResult, analyze_black_ring_frame};
pub use color::{ColorDetectOutput, run_color_detect_with_frame};
pub use config::{
    BlackRingDetectConfig, CameraDevice, ColorDetectConfig, CrossDetectConfig, QrDetectConfig,
};
#[cfg(test)]
pub use config::{CrossColor, TargetCorrection};
pub use cross::{CrossDetectOutput, format_cross_value, run_cross_detect_with_frame};
#[cfg(test)]
pub use cross::{CrossResult, analyze_cross_frame};
pub use qr::run_qr_detect;
