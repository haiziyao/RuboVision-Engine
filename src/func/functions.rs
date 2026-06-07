use std::thread::sleep;
use std::time::Duration;

use anyhow::{Result, anyhow};
use tracing::debug;

use crate::config::{ColorDetectParams, CrossDetectParams, DebugParams, QrDetectParams};
use crate::device::{
    CameraDevice, ColorDetectConfig, CrossDetectConfig, QrDetectConfig, run_color_detect,
    run_cross_detect, run_qr_detect,
};
use crate::func::{FunctionResult, NoDevice, ValidateParams, declare_functions};

impl ValidateParams for ColorDetectParams {
    fn validate(&self) -> Result<()> {
        if self.loop_count <= 0 {
            return Err(anyhow!("loop_count must be greater than 0"));
        }
        if !(0.0..=0.5).contains(&self.radius_ratio) || self.radius_ratio == 0.0 {
            return Err(anyhow!("radius_ratio must be in (0, 0.5]"));
        }
        if !(0.0..=1.0).contains(&self.detect_area_access_rate) {
            return Err(anyhow!("detect_area_access_rate must be in [0, 1]"));
        }
        if self.color_ranges.is_empty() {
            return Err(anyhow!("color_ranges must not be empty"));
        }
        for range in &self.color_ranges {
            let [h_min, h_max, s_min, s_max, v_min, v_max] = range.hsv;
            if range.name.trim().is_empty() {
                return Err(anyhow!("color range name must not be empty"));
            }
            if !(0..=179).contains(&h_min)
                || !(0..=179).contains(&h_max)
                || !(0..=255).contains(&s_min)
                || !(0..=255).contains(&s_max)
                || !(0..=255).contains(&v_min)
                || !(0..=255).contains(&v_max)
                || h_min > h_max
                || s_min > s_max
                || v_min > v_max
            {
                return Err(anyhow!("invalid HSV range for `{}`", range.name));
            }
        }
        Ok(())
    }
}

impl ValidateParams for QrDetectParams {
    fn validate(&self) -> Result<()> {
        Ok(())
    }
}

impl ValidateParams for CrossDetectParams {
    fn validate(&self) -> Result<()> {
        Ok(())
    }
}

impl ValidateParams for DebugParams {
    fn validate(&self) -> Result<()> {
        if self.message.trim().is_empty() {
            return Err(anyhow!("message must not be empty"));
        }
        Ok(())
    }
}

fn color_detect(params: &ColorDetectParams, camera: &CameraDevice) -> Result<FunctionResult> {
    let config = ColorDetectConfig::from_params(params, camera);
    let value = run_color_detect(&config)?;
    Ok(FunctionResult::value(
        format!("color_detect finished: {value}"),
        value,
    ))
}

fn qr_detect(params: &QrDetectParams, camera: &CameraDevice) -> Result<FunctionResult> {
    let config = QrDetectConfig::from_params(params, camera);
    let value = run_qr_detect(&config)?;
    Ok(FunctionResult::value(
        format!("qr_detect finished: {value}"),
        value.to_string(),
    ))
}

fn cross_detect(_params: &CrossDetectParams, camera: &CameraDevice) -> Result<FunctionResult> {
    let config = CrossDetectConfig::from_params(_params, camera);
    let value = run_cross_detect(&config)?;
    Ok(FunctionResult::value(
        format!("cross_detect finished: {value}"),
        value,
    ))
}

fn debug_fun(params: &DebugParams, _device: &NoDevice) -> Result<FunctionResult> {
    debug!("debug Function executing");
    sleep(Duration::from_secs(5));
    Ok(FunctionResult::ok(format!(
        "this is the debug function {}",
        params.message
    )))
}

declare_functions! {
    color_detect(params: ColorDetectParams, device: CameraDevice) => color_detect,
    qr_detect(params: QrDetectParams, device: CameraDevice) => qr_detect,
    cross_detect(params: CrossDetectParams, device: CameraDevice) => cross_detect,
    debug_fun(params: DebugParams, device: NoDevice) => debug_fun,
}
