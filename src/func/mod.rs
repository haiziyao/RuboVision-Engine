pub mod black_ring;
pub mod color;
pub mod cross;
pub mod debug;
pub mod qr;

pub use black_ring::BlackRingDetect;
pub use color::ColorDetect;
pub use cross::CrossDetect;
pub use debug::DebugFun;
pub use qr::QrDetect;

#[cfg(feature = "opencv")]
async fn read_frames(
    camera: &crate::device::CameraDevice,
    count: i32,
) -> Result<Vec<opencv::core::Mat>, rubo_engine::FunctionError> {
    let mut frames = Vec::with_capacity(count.max(1) as usize);
    for _ in 0..count.max(1) {
        frames.push(camera.frame().await?);
    }
    Ok(frames)
}
