use anyhow::Result;

use super::support::{
    black_ring_detect_config_from_config, color_detect_config_from_config, configured_color_names,
    configured_device_path, cross_detect_config_from_config, parse_cross_runtime_param,
    qr_detect_config_from_config,
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
    assert_eq!(config.target_correction.x, 0);
    assert_eq!(config.target_correction.y, 0);
    assert_eq!(
        config
            .colors
            .iter()
            .map(|color| color.id)
            .collect::<Vec<_>>(),
        vec![1, 2, 3, 4, 5]
    );
    Ok(())
}

#[test]
fn cross_runtime_param_accepts_only_zero_through_five() -> Result<()> {
    assert_eq!(parse_cross_runtime_param("0")?, 0);
    assert_eq!(parse_cross_runtime_param("5")?, 5);
    assert!(parse_cross_runtime_param("6").is_err());
    assert!(parse_cross_runtime_param("red").is_err());
    Ok(())
}
