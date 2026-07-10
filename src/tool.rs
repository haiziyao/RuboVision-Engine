#[cfg(feature = "opencv")]
use base64::{Engine as _, engine::general_purpose::STANDARD};

#[cfg(feature = "opencv")]
pub fn mat_to_jpeg_data_url(mat: &opencv::core::Mat) -> opencv::Result<String> {
    let mut bytes = opencv::types::VectorOfu8::new();
    opencv::imgcodecs::imencode(".jpg", mat, &mut bytes, &opencv::types::VectorOfi32::new())?;
    Ok(format!("data:image/jpeg;base64,{}", STANDARD.encode(bytes)))
}

#[cfg(not(feature = "opencv"))]
pub fn opencv_disabled_message(function_id: &str) -> String {
    format!("{function_id} requires the `opencv` feature and a local OpenCV runtime")
}
