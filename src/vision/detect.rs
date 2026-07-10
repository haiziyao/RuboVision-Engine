#[cfg(feature = "opencv")]
mod opencv_impl {
    use std::f64::consts::{PI, TAU};

    use opencv::{
        core::{self, Mat, Point, Point2f, Scalar, Size},
        imgproc,
        prelude::*,
        types,
    };

    pub struct VisionFrameOutput<T> {
        pub value: T,
        pub frame: Mat,
    }

    #[derive(Debug, Clone, serde::Deserialize)]
    pub struct ColorRange {
        pub name: String,
        pub hsv: [i32; 6],
    }

    #[derive(Debug, Clone)]
    pub struct ColorDetectConfig {
        pub loop_count: i32,
        pub radius_ratio: f64,
        pub detect_area_access_rate: f64,
        pub color_ranges: Vec<ColorRange>,
    }

    #[derive(Debug, Clone, Copy, serde::Deserialize)]
    #[serde(default)]
    pub struct TargetCorrection {
        pub x: i32,
        pub y: i32,
    }

    impl Default for TargetCorrection {
        fn default() -> Self {
            Self { x: 0, y: 0 }
        }
    }

    #[derive(Debug, Clone)]
    pub struct BlackRingDetectConfig {
        pub loop_count: i32,
        pub target_correction: TargetCorrection,
        pub black_threshold: i32,
        pub min_radius: f64,
        pub max_radius: f64,
        pub min_circularity: f64,
        pub min_score: u8,
    }

    #[derive(Debug, Clone, serde::Deserialize)]
    pub struct CrossColor {
        pub id: u8,
        pub name: String,
        pub hsv: [i32; 6],
        pub min_area: f64,
        pub min_circularity: f64,
    }

    #[derive(Debug, Clone)]
    pub struct CrossDetectConfig {
        pub loop_count: i32,
        pub target_correction: TargetCorrection,
        pub black_threshold: i32,
        pub close_kernel_size: i32,
        pub dilate_kernel_size: i32,
        pub dilate_iterations: i32,
        pub min_radius: f64,
        pub max_radius: f64,
        pub center_tolerance: f64,
        pub min_arc_points: usize,
        pub min_ring_score: u8,
        pub colors: Vec<CrossColor>,
    }

    pub fn detect_color(
        frames: Vec<Mat>,
        config: &ColorDetectConfig,
    ) -> opencv::Result<VisionFrameOutput<String>> {
        let mut best_color = "unknown".to_string();
        let mut best_ratio = -1.0_f64;
        let mut best_frame = None;
        for frame in frames {
            let (color, ratio) = detect_color_frame(&frame, config)?;
            if ratio > best_ratio {
                best_color = color;
                best_ratio = ratio;
                best_frame = Some(frame);
            }
        }
        let mut frame = best_frame.ok_or_else(no_frames_error)?;
        draw_color_overlay(&mut frame, &best_color, best_ratio, config.radius_ratio)?;
        Ok(VisionFrameOutput {
            value: best_color,
            frame,
        })
    }

    pub fn detect_qr(frames: Vec<Mat>) -> opencv::Result<VisionFrameOutput<String>> {
        let mut last_frame = None;
        for frame in frames {
            let gray = bgr_to_gray(&frame)?;
            let content = decode_qr(&gray)?;
            if !content.is_empty() {
                return Ok(VisionFrameOutput {
                    value: content,
                    frame,
                });
            }
            last_frame = Some(frame);
        }
        Ok(VisionFrameOutput {
            value: String::new(),
            frame: last_frame.ok_or_else(no_frames_error)?,
        })
    }

    pub fn detect_black_ring(
        frames: Vec<Mat>,
        config: &BlackRingDetectConfig,
    ) -> opencv::Result<VisionFrameOutput<String>> {
        let mut best_value = "RING,0,0,0,0".to_string();
        let mut best_score = 0_u8;
        let mut best_frame = None;
        for frame in frames {
            let result = analyze_black_ring_frame(&frame, config)?;
            if result.score >= best_score {
                best_score = result.score;
                best_value = result.value;
                best_frame = Some(result.frame);
            }
        }
        Ok(VisionFrameOutput {
            value: best_value,
            frame: best_frame.ok_or_else(no_frames_error)?,
        })
    }

    pub fn detect_cross(
        frames: Vec<Mat>,
        config: &CrossDetectConfig,
        runtime_param: u8,
    ) -> opencv::Result<VisionFrameOutput<String>> {
        let mut best_value = format!("CROSS,{runtime_param},0,0,0,0");
        let mut best_score = 0_u8;
        let mut best_frame = None;
        for frame in frames {
            let result = analyze_cross_frame(&frame, config, runtime_param)?;
            if result.score >= best_score {
                best_score = result.score;
                best_value = result.value;
                best_frame = Some(result.frame);
            }
        }
        Ok(VisionFrameOutput {
            value: best_value,
            frame: best_frame.ok_or_else(no_frames_error)?,
        })
    }

    struct ScoredFrame {
        value: String,
        score: u8,
        frame: Mat,
    }

    fn no_frames_error() -> opencv::Error {
        opencv::Error::new(core::StsError, "camera returned no frames".to_string())
    }

    fn bgr_to_gray(frame: &Mat) -> opencv::Result<Mat> {
        let mut gray = Mat::default();
        imgproc::cvt_color(frame, &mut gray, imgproc::COLOR_BGR2GRAY, 0)?;
        Ok(gray)
    }

    fn hsv_inrange(frame: &Mat, hsv: [i32; 6]) -> opencv::Result<Mat> {
        let mut hsv_frame = Mat::default();
        imgproc::cvt_color(frame, &mut hsv_frame, imgproc::COLOR_BGR2HSV, 0)?;
        let lower = Scalar::new(hsv[0] as f64, hsv[2] as f64, hsv[4] as f64, 0.0);
        let upper = Scalar::new(hsv[1] as f64, hsv[3] as f64, hsv[5] as f64, 0.0);
        let mut mask = Mat::default();
        core::in_range(&hsv_frame, &lower, &upper, &mut mask)?;
        Ok(mask)
    }

    fn detect_color_frame(
        frame: &Mat,
        config: &ColorDetectConfig,
    ) -> opencv::Result<(String, f64)> {
        let circle_mask = circle_mask(frame.size()?, config.radius_ratio)?;
        let total = core::count_non_zero(&circle_mask)? as f64;
        let mut best_name = "unknown".to_string();
        let mut best_ratio = 0.0;
        for color in &config.color_ranges {
            let mask = hsv_inrange(frame, color.hsv)?;
            let mut masked = Mat::default();
            core::bitwise_and(&mask, &mask, &mut masked, &circle_mask)?;
            let ratio = if total > 0.0 {
                core::count_non_zero(&masked)? as f64 / total
            } else {
                0.0
            };
            if ratio > best_ratio {
                best_ratio = ratio;
                best_name = color.name.clone();
            }
        }
        if best_ratio < config.detect_area_access_rate {
            best_name = "unknown".to_string();
        }
        Ok((best_name, best_ratio))
    }

    fn circle_mask(size: Size, radius_ratio: f64) -> opencv::Result<Mat> {
        let mut mask = Mat::zeros(size.height, size.width, core::CV_8UC1)?.to_mat()?;
        let center = Point::new(size.width / 2, size.height / 2);
        let radius = ((size.width.min(size.height) as f64) * radius_ratio) as i32;
        imgproc::circle(
            &mut mask,
            center,
            radius,
            Scalar::all(255.0),
            -1,
            imgproc::LINE_8,
            0,
        )?;
        Ok(mask)
    }

    fn draw_color_overlay(
        frame: &mut Mat,
        color: &str,
        ratio: f64,
        radius_ratio: f64,
    ) -> opencv::Result<()> {
        let size = frame.size()?;
        let center = Point::new(size.width / 2, size.height / 2);
        let radius = ((size.width.min(size.height) as f64) * radius_ratio) as i32;
        imgproc::circle(
            frame,
            center,
            radius,
            Scalar::new(0.0, 255.0, 0.0, 0.0),
            2,
            imgproc::LINE_8,
            0,
        )?;
        imgproc::put_text(
            frame,
            &format!("color: {color} ratio: {ratio:.2}"),
            Point::new(10, 30),
            imgproc::FONT_HERSHEY_SIMPLEX,
            0.8,
            Scalar::new(255.0, 255.0, 255.0, 0.0),
            2,
            imgproc::LINE_AA,
            false,
        )
    }

    fn decode_qr(gray: &Mat) -> opencv::Result<String> {
        let size = gray.size()?;
        let data = gray.data_bytes()?;
        let mut decoder = quircs::Quirc::default();
        let codes = decoder.identify(size.width as usize, size.height as usize, data);
        for code in codes.flatten() {
            if let Ok(decoded) = code.decode() {
                if let Ok(text) = std::str::from_utf8(&decoded.payload) {
                    return Ok(text.to_string());
                }
            }
        }
        Ok(String::new())
    }

    fn analyze_black_ring_frame(
        frame: &Mat,
        config: &BlackRingDetectConfig,
    ) -> opencv::Result<ScoredFrame> {
        let gray = bgr_to_gray(frame)?;
        let mut mask = Mat::default();
        imgproc::threshold(
            &gray,
            &mut mask,
            config.black_threshold as f64,
            255.0,
            imgproc::THRESH_BINARY_INV,
        )?;
        let mut contours = types::VectorOfVectorOfPoint::new();
        imgproc::find_contours(
            &mask,
            &mut contours,
            imgproc::RETR_EXTERNAL,
            imgproc::CHAIN_APPROX_SIMPLE,
            Point::new(0, 0),
        )?;
        let mut best_center = None;
        let mut best_radius = 0.0_f32;
        let mut best_score = 0_u8;
        for index in 0..contours.len() {
            let contour = contours.get(index)?;
            if contour.len() < 5 {
                continue;
            }
            let area = imgproc::contour_area(&contour, false)?;
            let perimeter = imgproc::arc_length(&contour, true)?;
            if area <= 0.0 || perimeter <= 0.0 {
                continue;
            }
            let mut center = Point2f::default();
            let mut radius = 0.0_f32;
            imgproc::min_enclosing_circle(&contour, &mut center, &mut radius)?;
            let radius64 = radius as f64;
            if radius64 < config.min_radius || radius64 > config.max_radius {
                continue;
            }
            let circularity = (4.0 * PI * area / (perimeter * perimeter)).clamp(0.0, 1.0);
            if circularity < config.min_circularity {
                continue;
            }
            let score = (circularity * 100.0).round().clamp(0.0, 100.0) as u8;
            if score >= config.min_score && score >= best_score {
                best_center = Some(center);
                best_radius = radius;
                best_score = score;
            }
        }
        let mut annotated = frame.clone();
        let size = frame.size()?;
        let target = Point::new(
            size.width / 2 + config.target_correction.x,
            size.height / 2 + config.target_correction.y,
        );
        draw_marker(&mut annotated, target, Scalar::new(255.0, 0.0, 0.0, 0.0))?;
        let value = if let Some(center) = best_center {
            let center_point = Point::new(center.x.round() as i32, center.y.round() as i32);
            imgproc::circle(
                &mut annotated,
                center_point,
                best_radius.round() as i32,
                Scalar::new(0.0, 255.0, 0.0, 0.0),
                2,
                imgproc::LINE_AA,
                0,
            )?;
            draw_marker(
                &mut annotated,
                center_point,
                Scalar::new(0.0, 255.0, 0.0, 0.0),
            )?;
            format!(
                "RING,1,{},{},{}",
                center_point.x - target.x,
                center_point.y - target.y,
                best_score
            )
        } else {
            "RING,0,0,0,0".to_string()
        };
        Ok(ScoredFrame {
            value,
            score: best_score,
            frame: annotated,
        })
    }

    fn analyze_cross_frame(
        frame: &Mat,
        config: &CrossDetectConfig,
        runtime_param: u8,
    ) -> opencv::Result<ScoredFrame> {
        let gray = bgr_to_gray(frame)?;
        let mut black = Mat::default();
        imgproc::threshold(
            &gray,
            &mut black,
            config.black_threshold as f64,
            255.0,
            imgproc::THRESH_BINARY_INV,
        )?;
        let black = apply_cross_mask_morphology(&black, config)?;
        let candidates = ring_candidates(&black, config)?;
        let ring = best_ring(&candidates, config);
        let size = frame.size()?;
        let mut annotated = frame.clone();
        let target = Point::new(
            size.width / 2 + config.target_correction.x,
            size.height / 2 + config.target_correction.y,
        );
        draw_marker(&mut annotated, target, Scalar::new(255.0, 0.0, 0.0, 0.0))?;
        let Some((ring_center, ring_radius, ring_score)) = ring else {
            return Ok(ScoredFrame {
                value: format!("CROSS,{runtime_param},0,0,0,0"),
                score: 0,
                frame: annotated,
            });
        };
        let ring_point = Point::new(ring_center.x.round() as i32, ring_center.y.round() as i32);
        draw_marker(
            &mut annotated,
            ring_point,
            Scalar::new(0.0, 255.0, 0.0, 0.0),
        )?;
        if runtime_param == 0 {
            let value = format!(
                "CROSS,0,1,{},{},{}",
                ring_point.x - target.x,
                ring_point.y - target.y,
                ring_score
            );
            return Ok(ScoredFrame {
                value,
                score: ring_score,
                frame: annotated,
            });
        }
        let Some(color) = config.colors.iter().find(|color| color.id == runtime_param) else {
            return Ok(ScoredFrame {
                value: format!("CROSS,{runtime_param},0,0,0,0"),
                score: 0,
                frame: annotated,
            });
        };
        let cylinder = best_colored_cylinder(frame, color, ring_center, ring_radius)?;
        let Some((center, cylinder_score)) = cylinder else {
            return Ok(ScoredFrame {
                value: format!("CROSS,{runtime_param},0,0,0,0"),
                score: ring_score,
                frame: annotated,
            });
        };
        let cylinder_point = Point::new(center.x.round() as i32, center.y.round() as i32);
        draw_marker(
            &mut annotated,
            cylinder_point,
            Scalar::new(0.0, 255.0, 255.0, 0.0),
        )?;
        let score = ring_score.min(cylinder_score);
        Ok(ScoredFrame {
            value: format!(
                "CROSS,{runtime_param},1,{},{},{}",
                cylinder_point.x - ring_point.x,
                cylinder_point.y - ring_point.y,
                score
            ),
            score,
            frame: annotated,
        })
    }

    fn ring_candidates(
        black_mask: &Mat,
        config: &CrossDetectConfig,
    ) -> opencv::Result<Vec<(Point2f, f32, u8)>> {
        let mut contours = types::VectorOfVectorOfPoint::new();
        imgproc::find_contours(
            black_mask,
            &mut contours,
            imgproc::RETR_LIST,
            imgproc::CHAIN_APPROX_NONE,
            Point::new(0, 0),
        )?;
        let mut out = Vec::new();
        for index in 0..contours.len() {
            let contour = contours.get(index)?;
            if contour.len() < config.min_arc_points {
                continue;
            }
            let mut center = Point2f::default();
            let mut radius = 0.0_f32;
            imgproc::min_enclosing_circle(&contour, &mut center, &mut radius)?;
            let radius64 = radius as f64;
            if radius64 < config.min_radius || radius64 > config.max_radius {
                continue;
            }
            let area = imgproc::contour_area(&contour, false)?;
            let perimeter = imgproc::arc_length(&contour, false)?;
            if perimeter <= 0.0 {
                continue;
            }
            let score = ((area / (PI * radius64 * radius64)).clamp(0.0, 1.0) * 100.0) as u8;
            if score >= config.min_ring_score {
                out.push((center, radius, score));
            }
        }
        Ok(out)
    }

    fn apply_cross_mask_morphology(
        black_mask: &Mat,
        config: &CrossDetectConfig,
    ) -> opencv::Result<Mat> {
        let close_kernel_size = config.close_kernel_size.max(1);
        let dilate_kernel_size = config.dilate_kernel_size.max(1);
        let close_kernel = imgproc::get_structuring_element(
            imgproc::MORPH_ELLIPSE,
            Size::new(close_kernel_size, close_kernel_size),
            Point::new(-1, -1),
        )?;
        let dilate_kernel = imgproc::get_structuring_element(
            imgproc::MORPH_ELLIPSE,
            Size::new(dilate_kernel_size, dilate_kernel_size),
            Point::new(-1, -1),
        )?;
        let mut closed = Mat::default();
        imgproc::morphology_ex(
            black_mask,
            &mut closed,
            imgproc::MORPH_CLOSE,
            &close_kernel,
            Point::new(-1, -1),
            1,
            core::BORDER_CONSTANT,
            imgproc::morphology_default_border_value()?,
        )?;
        let mut dilated = Mat::default();
        imgproc::dilate(
            &closed,
            &mut dilated,
            &dilate_kernel,
            Point::new(-1, -1),
            config.dilate_iterations.max(0),
            core::BORDER_CONSTANT,
            imgproc::morphology_default_border_value()?,
        )?;
        Ok(dilated)
    }

    fn best_ring(
        candidates: &[(Point2f, f32, u8)],
        config: &CrossDetectConfig,
    ) -> Option<(Point2f, f32, u8)> {
        candidates
            .iter()
            .copied()
            .max_by_key(|candidate| group_score(*candidate, candidates, config))
    }

    fn group_score(
        seed: (Point2f, f32, u8),
        candidates: &[(Point2f, f32, u8)],
        config: &CrossDetectConfig,
    ) -> u8 {
        let count = candidates
            .iter()
            .filter(|candidate| point_distance(seed.0, candidate.0) <= config.center_tolerance)
            .count();
        if count < 3 {
            0
        } else {
            seed.2.saturating_add((count as u8).saturating_mul(8))
        }
    }

    fn best_colored_cylinder(
        frame: &Mat,
        color: &CrossColor,
        ring_center: Point2f,
        ring_radius: f32,
    ) -> opencv::Result<Option<(Point2f, u8)>> {
        let mask = hsv_inrange(frame, color.hsv)?;
        let mut contours = types::VectorOfVectorOfPoint::new();
        imgproc::find_contours(
            &mask,
            &mut contours,
            imgproc::RETR_EXTERNAL,
            imgproc::CHAIN_APPROX_SIMPLE,
            Point::new(0, 0),
        )?;
        let mut best = None;
        let mut best_score = 0;
        for index in 0..contours.len() {
            let contour = contours.get(index)?;
            if contour.len() < 5 {
                continue;
            }
            let area = imgproc::contour_area(&contour, false)?;
            if area < color.min_area {
                continue;
            }
            let perimeter = imgproc::arc_length(&contour, true)?;
            if perimeter <= 0.0 {
                continue;
            }
            let circularity = (4.0 * PI * area / (perimeter * perimeter)).clamp(0.0, 1.0);
            if circularity < color.min_circularity {
                continue;
            }
            let moments = imgproc::moments(&contour, false)?;
            if moments.m00.abs() < f64::EPSILON {
                continue;
            }
            let center = Point2f::new(
                (moments.m10 / moments.m00) as f32,
                (moments.m01 / moments.m00) as f32,
            );
            if point_distance(center, ring_center) > ring_radius as f64 * 1.15 {
                continue;
            }
            let score = (circularity * 100.0).round().clamp(0.0, 100.0) as u8;
            if score > best_score {
                best_score = score;
                best = Some((center, score));
            }
        }
        Ok(best)
    }

    fn point_distance(left: Point2f, right: Point2f) -> f64 {
        (left.x as f64 - right.x as f64).hypot(left.y as f64 - right.y as f64)
    }

    fn draw_marker(frame: &mut Mat, center: Point, color: Scalar) -> opencv::Result<()> {
        let len = 12;
        imgproc::line(
            frame,
            Point::new(center.x - len, center.y),
            Point::new(center.x + len, center.y),
            color,
            2,
            imgproc::LINE_AA,
            0,
        )?;
        imgproc::line(
            frame,
            Point::new(center.x, center.y - len),
            Point::new(center.x, center.y + len),
            color,
            2,
            imgproc::LINE_AA,
            0,
        )
    }
}

#[cfg(feature = "opencv")]
pub use opencv_impl::*;
