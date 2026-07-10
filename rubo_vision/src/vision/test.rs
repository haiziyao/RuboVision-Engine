#[cfg(feature = "opencv")]
pub fn show_frame(window: &str, frame: &opencv::core::Mat) -> opencv::Result<()> {
    opencv::highgui::imshow(window, frame)?;
    opencv::highgui::wait_key(0)?;
    Ok(())
}
