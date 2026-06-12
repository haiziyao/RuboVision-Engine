use std::collections::HashSet;
use std::thread::sleep;
use std::time::Duration;

use anyhow::{Result, anyhow};
use tracing::debug;

use crate::config::{
    BlackRingDetectParams, ColorDetectParams, CrossDetectParams, DebugParams, QrDetectParams,
};
use crate::device::{
    BlackRingDetectConfig, BlackRingDetectOutput, CameraDevice, ColorDetectConfig,
    ColorDetectOutput, CrossDetectConfig, CrossDetectOutput, QrDetectConfig, QrDetectOutput,
    format_black_ring_value, format_cross_value, run_black_ring_detect_with_frame,
    run_color_detect_with_frame, run_cross_detect_with_frame, run_qr_detect_with_frame,
};
use crate::func::{FunctionResult, NoDevice, ValidateParams, declare_functions};
use crate::utils::web_tools::mat_to_jpeg_data_url;

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

impl ValidateParams for BlackRingDetectParams {
    fn validate(&self) -> Result<()> {
        if self.loop_count <= 0 {
            return Err(anyhow!("loop_count must be greater than 0"));
        }
        if !(0..=255).contains(&self.black_threshold) {
            return Err(anyhow!("black_threshold must be in [0, 255]"));
        }
        if self.min_radius <= 0.0 {
            return Err(anyhow!("min_radius must be greater than 0"));
        }
        if self.max_radius < self.min_radius {
            return Err(anyhow!(
                "max_radius must be greater than or equal to min_radius"
            ));
        }
        if !(0.0..=1.0).contains(&self.min_circularity) {
            return Err(anyhow!("min_circularity must be in [0, 1]"));
        }
        if self.min_score > 100 {
            return Err(anyhow!("min_score must be in [0, 100]"));
        }
        Ok(())
    }
}

impl ValidateParams for CrossDetectParams {
    fn validate(&self) -> Result<()> {
        if self.loop_count <= 0 {
            return Err(anyhow!("loop_count must be greater than 0"));
        }
        if !(0..=255).contains(&self.black_threshold) {
            return Err(anyhow!("black_threshold must be in [0, 255]"));
        }
        if !valid_morphology_kernel(self.close_kernel_size) {
            return Err(anyhow!(
                "close_kernel_size must be an odd integer in [1, 31]"
            ));
        }
        if !valid_morphology_kernel(self.dilate_kernel_size) {
            return Err(anyhow!(
                "dilate_kernel_size must be an odd integer in [1, 31]"
            ));
        }
        if !(0..=5).contains(&self.dilate_iterations) {
            return Err(anyhow!("dilate_iterations must be in [0, 5]"));
        }
        if self.min_radius <= 0.0 {
            return Err(anyhow!("min_radius must be greater than 0"));
        }
        if self.max_radius < self.min_radius {
            return Err(anyhow!(
                "max_radius must be greater than or equal to min_radius"
            ));
        }
        if self.center_tolerance <= 0.0 {
            return Err(anyhow!("center_tolerance must be greater than 0"));
        }
        if self.min_arc_points < 5 {
            return Err(anyhow!("min_arc_points must be at least 5"));
        }
        if self.min_ring_score > 100 {
            return Err(anyhow!("min_ring_score must be in [0, 100]"));
        }

        let ids: HashSet<u8> = self.colors.iter().map(|color| color.id).collect();
        if ids != HashSet::from([1, 2, 3, 4, 5]) || self.colors.len() != 5 {
            return Err(anyhow!("cross colors must contain unique ids 1 through 5"));
        }

        for color in &self.colors {
            if color.name.trim().is_empty() {
                return Err(anyhow!("cross color name must not be empty"));
            }
            if color.min_area <= 0.0 {
                return Err(anyhow!(
                    "cross color `{}` min_area must be greater than 0",
                    color.name
                ));
            }
            if !(0.0..=1.0).contains(&color.min_circularity) {
                return Err(anyhow!(
                    "cross color `{}` min_circularity must be in [0, 1]",
                    color.name
                ));
            }

            let [h_min, h_max, s_min, s_max, v_min, v_max] = color.hsv;
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
                return Err(anyhow!("invalid HSV range for `{}`", color.name));
            }
        }
        Ok(())
    }
}

fn valid_morphology_kernel(size: i32) -> bool {
    (1..=31).contains(&size) && size % 2 == 1
}

impl ValidateParams for DebugParams {
    fn validate(&self) -> Result<()> {
        if self.message.trim().is_empty() {
            return Err(anyhow!("message must not be empty"));
        }
        Ok(())
    }
}

fn color_detect(
    params: &ColorDetectParams,
    _runtime_param: u8,
    camera: &CameraDevice,
) -> Result<FunctionResult> {
    let config = ColorDetectConfig::from_params(params, camera);
    let value = run_color_detect_with_frame(&config)?;
    color_detect_output_to_function_result(value)
}

fn color_detect_output_to_function_result(output: ColorDetectOutput) -> Result<FunctionResult> {
    let image = mat_to_jpeg_data_url(&output.frame)?;
    Ok(FunctionResult::value_with_image(
        format!("color_detect finished: {}", output.color),
        output.color,
        image,
    ))
}

fn qr_detect(
    params: &QrDetectParams,
    _runtime_param: u8,
    camera: &CameraDevice,
) -> Result<FunctionResult> {
    let config = QrDetectConfig::from_params(params, camera);
    let output = run_qr_detect_with_frame(&config)?;
    qr_detect_output_to_function_result(output)
}

fn qr_detect_output_to_function_result(output: QrDetectOutput) -> Result<FunctionResult> {
    let image = mat_to_jpeg_data_url(&output.frame)?;
    Ok(FunctionResult::value_with_image(
        format!("qr_detect finished: {}", output.value),
        output.value.to_string(),
        image,
    ))
}

fn black_ring_detect(
    params: &BlackRingDetectParams,
    _runtime_param: u8,
    camera: &CameraDevice,
) -> Result<FunctionResult> {
    let config = BlackRingDetectConfig::from_params(params, camera);
    let value = run_black_ring_detect_with_frame(&config)?;
    black_ring_output_to_function_result(value)
}

fn black_ring_output_to_function_result(output: BlackRingDetectOutput) -> Result<FunctionResult> {
    let image = mat_to_jpeg_data_url(&output.frame)?;
    let value = format_black_ring_value(&output.result);
    Ok(FunctionResult::value_with_image(
        format!("black_ring_detect finished: {value}"),
        value,
        image,
    ))
}

fn cross(
    params: &CrossDetectParams,
    runtime_param: u8,
    camera: &CameraDevice,
) -> Result<FunctionResult> {
    let config = CrossDetectConfig::from_params(params, camera);
    let output = run_cross_detect_with_frame(runtime_param, &config)?;
    cross_output_to_function_result(output)
}

fn cross_output_to_function_result(output: CrossDetectOutput) -> Result<FunctionResult> {
    let image = mat_to_jpeg_data_url(&output.frame)?;
    let value = format_cross_value(&output.result);
    Ok(FunctionResult::value_with_image(
        format!("cross finished: {value}"),
        value,
        image,
    ))
}

fn debug_fun(
    params: &DebugParams,
    _runtime_param: u8,
    _device: &NoDevice,
) -> Result<FunctionResult> {
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
    black_ring_detect(params: BlackRingDetectParams, device: CameraDevice) => black_ring_detect,
    cross(params: CrossDetectParams, device: CameraDevice) => cross,
    debug_fun(params: DebugParams, device: NoDevice) => debug_fun,
}

#[cfg(test)]
mod tests {
    use opencv::{core, prelude::*};

    use crate::device::{BlackRingResult, ColorDetectOutput, CrossDetectOutput, CrossResult};

    use super::*;

    #[test]
    fn color_detect_output_to_function_result_attaches_image() -> Result<()> {
        let frame = Mat::new_rows_cols_with_default(
            8,
            8,
            core::CV_8UC3,
            core::Scalar::new(0.0, 0.0, 255.0, 0.0),
        )?;
        let result = color_detect_output_to_function_result(ColorDetectOutput {
            color: "red".to_string(),
            frame,
        })?;

        assert_eq!(result.text, "color_detect finished: red");
        assert_eq!(result.value.as_deref(), Some("red"));
        assert!(
            result
                .image
                .as_deref()
                .is_some_and(|image| image.starts_with("data:image/jpeg;base64,"))
        );
        Ok(())
    }

    #[test]
    fn qr_detect_output_to_function_result_keeps_value_and_image() -> Result<()> {
        let frame = Mat::new_rows_cols_with_default(
            8,
            8,
            core::CV_8UC3,
            core::Scalar::new(255.0, 255.0, 255.0, 0.0),
        )?;
        let result = qr_detect_output_to_function_result(QrDetectOutput { value: 42, frame })?;

        assert_eq!(result.text, "qr_detect finished: 42");
        assert_eq!(result.value.as_deref(), Some("42"));
        assert!(
            result
                .image
                .as_deref()
                .is_some_and(|image| image.starts_with("data:image/jpeg;base64,"))
        );
        Ok(())
    }

    #[test]
    fn black_ring_output_to_function_result_keeps_uart_value_and_image() -> Result<()> {
        let frame = Mat::new_rows_cols_with_default(
            8,
            8,
            core::CV_8UC3,
            core::Scalar::new(255.0, 255.0, 255.0, 0.0),
        )?;
        let result = black_ring_output_to_function_result(BlackRingDetectOutput {
            result: BlackRingResult {
                valid: true,
                center: None,
                radius: 0.0,
                dx: -42,
                dy: 18,
                score: 87,
            },
            frame,
        })?;

        assert_eq!(result.text, "black_ring_detect finished: RING,1,-42,18,87");
        assert_eq!(result.value.as_deref(), Some("RING,1,-42,18,87"));
        assert!(
            result
                .image
                .as_deref()
                .is_some_and(|image| image.starts_with("data:image/jpeg;base64,"))
        );
        Ok(())
    }

    #[test]
    fn cross_output_to_function_result_keeps_uart_value_and_image() -> Result<()> {
        let frame = Mat::new_rows_cols_with_default(8, 8, core::CV_8UC3, core::Scalar::all(255.0))?;
        let result = cross_output_to_function_result(CrossDetectOutput {
            result: CrossResult {
                param: 1,
                valid: true,
                ring_center: None,
                cylinder_center: None,
                dx: 20,
                dy: -8,
                score: 88,
            },
            frame,
        })?;

        assert_eq!(result.text, "cross finished: CROSS,1,1,20,-8,88");
        assert_eq!(result.value.as_deref(), Some("CROSS,1,1,20,-8,88"));
        assert!(
            result
                .image
                .as_deref()
                .is_some_and(|image| image.starts_with("data:image/jpeg;base64,"))
        );
        Ok(())
    }
}
