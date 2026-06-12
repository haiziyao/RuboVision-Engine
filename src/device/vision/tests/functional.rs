use anyhow::Result;

use super::super::{
    format_cross_value, run_color_detect_with_frame, run_cross_detect_with_frame,
    run_qr_detect_with_frame,
};
use super::support::{
    color_detect_config_from_config, cross_detect_config_from_config, qr_detect_config_from_config,
};

#[test]
#[ignore = "requires color camera and GUI; reads only config and runs color_detect"]
fn color_detect_function_from_config() -> Result<()> {
    let config = color_detect_config_from_config()?;

    let result = run_color_detect_with_frame(&config)?;
    println!("color_detect result: {}", result.color);
    Ok(())
}

#[test]
#[ignore = "requires QR camera and GUI; reads only config and runs qr_detect"]
fn qr_detect_function_from_config() -> Result<()> {
    let config = qr_detect_config_from_config()?;

    let output = run_qr_detect_with_frame(&config)?;
    println!("qr_detect result: {}", output.value);
    Ok(())
}

#[test]
#[ignore = "requires cross camera and GUI; reads only config and runs cross"]
fn cross_detect_function_from_config() -> Result<()> {
    let config = cross_detect_config_from_config()?;
    let output = run_cross_detect_with_frame(0, &config)?;
    let result = format_cross_value(&output.result);

    println!("cross result: {result}");
    Ok(())
}
