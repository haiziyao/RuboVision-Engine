use anyhow::Result;

use super::support::{
    black_ring_detect_config_from_config, color_detect_config_from_config, configured_color_names,
    configured_device_path, cross_detect_config_from_config, qr_detect_config_from_config,
};

#[test]
fn color_detect_config_from_config_file() -> Result<()> {
    let config = color_detect_config_from_config()?;
    let names: Vec<String> = config
        .color_ranges
        .iter()
        .map(|range| range.name.clone())
        .collect();

    assert_eq!(config.path, configured_device_path("color_camera")?);
    assert_eq!(names, configured_color_names()?);
    Ok(())
}

#[test]
fn qr_detect_config_from_config_file() -> Result<()> {
    let config = qr_detect_config_from_config()?;

    assert_eq!(config.path, configured_device_path("qr_camera")?);
    Ok(())
}

#[test]
fn black_ring_detect_config_from_config_file() -> Result<()> {
    let config = black_ring_detect_config_from_config()?;

    assert_eq!(config.path, configured_device_path("color_camera")?);
    assert!(config.loop_count > 0);
    Ok(())
}

#[test]
fn cross_detect_config_from_config_file() -> Result<()> {
    let config = cross_detect_config_from_config()?;

    assert_eq!(config.path, configured_device_path("cross_camera")?);
    Ok(())
}
