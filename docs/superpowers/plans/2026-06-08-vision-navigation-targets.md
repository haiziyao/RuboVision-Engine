# Vision Navigation Targets Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add reusable ground coordinate conversion plus independent `cross_detect`, `lane_detect`, and `inner_circle_detect` vision functions with UART/Web outputs.

**Architecture:** Keep each detector in its own `src/device/vision/*.rs` module. Share only small data types and the perspective conversion helper. Each function returns a typed detection result, and `src/func/functions.rs` formats it into `TaskOutput.value_with_image` so UART receives only the compact string while Web receives the annotated frame.

**Tech Stack:** Rust, OpenCV crate, Serde/TOML typed params, existing `TaskOutput`, existing ignored GUI tests with HighGUI.

---

## File Structure

- Create `src/utils/ground_transform.rs`: perspective transform params, validation, point conversion, and pixel fallback helpers.
- Modify `src/utils/mod.rs`: export `ground_transform`.
- Modify `src/config/type.rs`: add common point/ground-transform config and params for cross/lane/inner circle.
- Modify `src/device/vision/config.rs`: add runtime configs for cross/lane/inner circle.
- Modify `src/device/vision/camera.rs`: reuse camera opening for all camera-based detectors.
- Replace `src/device/vision/cross.rs`: implement cross analysis and output.
- Create `src/device/vision/lane.rs`: implement dual black-line analysis and output.
- Create `src/device/vision/inner_circle.rs`: implement innermost circle analysis and output.
- Modify `src/device/vision.rs`: export new detector types/functions.
- Modify `src/func/functions.rs`: validate new params and format `CROSS`, `LANE`, `INNER_CIRCLE`.
- Modify `src/func/register.rs`: include `lane_detect` and `inner_circle_detect`.
- Modify `config/functions.toml`: add/tune params and return targets.
- Modify `config/bindings.toml`: add debug and UART bindings for new functions.
- Modify `src/device/vision/tests/*.rs`: add unit and ignored GUI tests.

Do not implement `color_cylinder_detect` in this plan.

---

## Task 1: Ground Transform Utility

**Files:**
- Create: `src/utils/ground_transform.rs`
- Modify: `src/utils/mod.rs`
- Test: `src/utils/ground_transform.rs`

- [ ] **Step 1: Write failing unit tests**

Create tests for identity-like transform and pixel fallback semantics:

```rust
#[test]
fn ground_transform_maps_image_point_to_ground_point() -> Result<()> {
    let transform = GroundTransform::from_points(
        &[[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]],
        &[[-50.0, 100.0], [50.0, 100.0], [50.0, 0.0], [-50.0, 0.0]],
    )?;

    let point = transform.map_point(Point2f::new(50.0, 50.0))?;
    assert!((point.lateral - 0.0).abs() < 0.01);
    assert!((point.forward - 50.0).abs() < 0.01);
    Ok(())
}

#[test]
fn pixel_offset_uses_vehicle_sign_convention() {
    let offset = pixel_offset(
        Point2f::new(80.0, 20.0),
        core::Size::new(100, 100),
        TargetCorrection { x: 0, y: 0 },
    );
    assert_eq!(offset.lateral.round() as i32, 30);
    assert_eq!(offset.forward.round() as i32, 30);
}
```

- [ ] **Step 2: Run RED**

Run:

```bash
cargo test ground_transform -- --nocapture
```

Expected: compile fails because `GroundTransform` and `pixel_offset` do not exist.

- [ ] **Step 3: Implement utility**

Implement:

```rust
pub struct GroundPoint {
    pub lateral: f64,
    pub forward: f64,
}

pub struct GroundTransform {
    matrix: Mat,
}

impl GroundTransform {
    pub fn from_points(image_points: &[[f64; 2]], ground_points_mm: &[[f64; 2]]) -> Result<Self>;
    pub fn map_point(&self, point: Point2f) -> Result<GroundPoint>;
}

pub fn pixel_offset(point: Point2f, size: core::Size, correction: TargetCorrection) -> GroundPoint;
```

Use OpenCV perspective transform with `get_perspective_transform` for four points. If more than four points are later needed, add a separate calibration task instead of expanding this implementation.

- [ ] **Step 4: Run GREEN**

Run:

```bash
cargo test ground_transform -- --nocapture
cargo fmt
```

Expected: tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/utils/mod.rs src/utils/ground_transform.rs
git commit -m "feat: add ground transform utility"
```

---

## Task 2: Shared Detection Result Formatting

**Files:**
- Modify: `src/config/type.rs`
- Modify: `src/device/vision/config.rs`
- Modify: `src/func/functions.rs`
- Test: `src/func/functions.rs`

- [ ] **Step 1: Write failing formatter tests**

Add tests for each UART string:

```rust
#[test]
fn cross_output_formats_mm_value() {
    let value = format_cross_value(&CrossDetectResult {
        valid: true,
        unit: OutputUnit::Mm,
        lateral: -120,
        forward: 850,
        yaw_cdeg: 1530,
        score: 91,
    });
    assert_eq!(value, "CROSS,1,MM,-120,850,1530,91");
}

#[test]
fn lane_output_formats_px_value() {
    let value = format_lane_value(&LaneDetectResult {
        valid: true,
        unit: OutputUnit::Px,
        lateral: 23,
        heading_cdeg: -540,
        width: 310,
        score: 82,
    });
    assert_eq!(value, "LANE,1,PX,23,-540,310,82");
}

#[test]
fn inner_circle_invalid_formats_na() {
    assert_eq!(
        format_inner_circle_value(&InnerCircleDetectResult::invalid()),
        "INNER_CIRCLE,0,NA,0,0,0"
    );
}
```

- [ ] **Step 2: Run RED**

Run:

```bash
cargo test output_formats -- --nocapture
```

Expected: compile fails because result structs and formatters do not exist.

- [ ] **Step 3: Implement shared config and result types**

Add reusable config:

```rust
pub struct TargetCorrectionConfig {
    pub x: i32,
    pub y: i32,
}

pub struct GroundTransformParams {
    pub image_points: [[f64; 2]; 4],
    pub ground_points_mm: [[f64; 2]; 4],
}
```

For each detector params struct, include:

```rust
pub debug_model: bool,
pub loop_count: i32,
pub use_ground_transform: bool,
pub target_correction: TargetCorrectionConfig,
pub ground_transform: Option<GroundTransformParams>,
```

Validation rule: when `use_ground_transform = true`, `ground_transform` must be present.

- [ ] **Step 4: Implement formatter functions**

Add:

```rust
pub enum OutputUnit { Px, Mm, Na }
pub fn format_cross_value(result: &CrossDetectResult) -> String;
pub fn format_lane_value(result: &LaneDetectResult) -> String;
pub fn format_inner_circle_value(result: &InnerCircleDetectResult) -> String;
```

Do not change router behavior.

- [ ] **Step 5: Run GREEN**

Run:

```bash
cargo test output_formats -- --nocapture
cargo fmt
```

Expected: tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/config/type.rs src/device/vision/config.rs src/func/functions.rs
git commit -m "feat: add vision target output formats"
```

---

## Task 3: Cross Detection

**Files:**
- Modify: `src/device/vision/cross.rs`
- Modify: `src/device/vision.rs`
- Modify: `src/device/vision/tests/functional.rs`
- Modify: `src/device/vision/tests/cv_steps.rs`
- Test: `src/device/vision/tests/cross.rs` or `src/device/vision/cross.rs`

- [ ] **Step 1: Write failing synthetic cross tests**

Use a generated tilted plus sign:

```rust
#[test]
fn cross_detects_tilted_plus_center_and_value() -> Result<()> {
    let frame = synthetic_cross_frame(640, 480, Point::new(320, 260), 12.0)?;
    let config = CrossDetectConfig::test_px();
    let analysis = analyze_cross_frame(&frame, &config)?;

    assert!(analysis.result.valid);
    assert!((analysis.result.lateral).abs() <= 4);
    assert!((analysis.result.forward + 20).abs() <= 6);
    assert!(analysis.result.score >= 60);
    assert_eq!(format_cross_value(&analysis.result).split(',').next(), Some("CROSS"));
    Ok(())
}
```

- [ ] **Step 2: Run RED**

Run:

```bash
cargo test cross_detects_tilted_plus_center_and_value -- --nocapture
```

Expected: fails because cross detection is still a stub.

- [ ] **Step 3: Implement `analyze_cross_frame`**

Implementation outline:

1. grayscale
2. black threshold
3. morphological close
4. find contours
5. reject tiny/huge contours
6. compute bounding rect, contour area, fill ratio
7. use skeleton-like scan or contour moments to find center
8. fit orientation with `min_area_rect` or PCA
9. score by area, plus-like fill ratio, orthogonal arm evidence
10. annotate original frame

Keep exported API:

```rust
pub fn analyze_cross_frame(frame_bgr: &Mat, config: &CrossDetectConfig) -> Result<CrossFrameAnalysis>;
pub fn run_cross_detect_with_frame(config: &CrossDetectConfig) -> Result<CrossDetectOutput>;
```

- [ ] **Step 4: Format function output**

Modify `cross_detect` in `src/func/functions.rs` to return `TaskOutput::value_with_image` with `CROSS,...`.

- [ ] **Step 5: Add ignored GUI test**

Add `show_cross_detect_cv_steps_from_config` to `src/device/vision/tests/cv_steps.rs`. Show frame, gray, black mask, candidate mask, annotated output.

- [ ] **Step 6: Run GREEN**

Run:

```bash
cargo test cross_detects_tilted_plus_center_and_value -- --nocapture
cargo test cross_detect -- --nocapture
cargo fmt
```

Expected: target tests pass; ignored GUI test is listed but not run.

- [ ] **Step 7: Commit**

```bash
git add src/device/vision/cross.rs src/device/vision.rs src/device/vision/tests src/func/functions.rs
git commit -m "feat: implement cross target detection"
```

---

## Task 4: Lane Detection

**Files:**
- Create: `src/device/vision/lane.rs`
- Modify: `src/device/vision.rs`
- Modify: `src/device/vision/config.rs`
- Modify: `src/config/type.rs`
- Modify: `src/func/functions.rs`
- Modify: `src/func/register.rs`
- Modify: `config/functions.toml`
- Modify: `config/bindings.toml`
- Test: `src/device/vision/tests/lane.rs`

- [ ] **Step 1: Write failing synthetic lane tests**

Create a synthetic frame with two curved black boundaries:

```rust
#[test]
fn lane_detects_center_heading_and_width() -> Result<()> {
    let frame = synthetic_lane_frame(640, 480)?;
    let config = LaneDetectConfig::test_px();
    let analysis = analyze_lane_frame(&frame, &config)?;

    assert!(analysis.result.valid);
    assert!(analysis.result.lateral.abs() <= 8);
    assert!(analysis.result.width > 150);
    assert!(analysis.result.score >= 60);
    assert_eq!(format_lane_value(&analysis.result).starts_with("LANE,1,PX"), true);
    Ok(())
}
```

- [ ] **Step 2: Run RED**

Run:

```bash
cargo test lane_detects_center_heading_and_width -- --nocapture
```

Expected: fails because `lane` module does not exist.

- [ ] **Step 3: Implement lane analysis**

Implementation outline:

1. grayscale
2. black threshold
3. lower/middle ROI
4. row scanning to find left/right black runs
5. fit left/right boundary lines using collected points
6. compute lane center at `lookahead_ratio`
7. compute heading from centerline slope
8. compute width at lookahead
9. score by number of rows, boundary symmetry, fit residual, width consistency
10. annotate boundaries, centerline, and lookahead point

Expose:

```rust
pub fn analyze_lane_frame(frame_bgr: &Mat, config: &LaneDetectConfig) -> Result<LaneFrameAnalysis>;
pub fn run_lane_detect_with_frame(config: &LaneDetectConfig) -> Result<LaneDetectOutput>;
```

- [ ] **Step 4: Register function**

Add `LaneDetectParams`, `LaneDetectConfig`, validation, `lane_detect` in `functions.rs`, and registry entry.

Default output:

```text
LANE,valid,unit,lateral,heading_cdeg,width,score
```

- [ ] **Step 5: Add ignored GUI test**

Add `show_lane_detect_cv_steps_from_config` to `cv_steps.rs`.

- [ ] **Step 6: Run GREEN**

Run:

```bash
cargo test lane_detects_center_heading_and_width -- --nocapture
cargo test lane_detect -- --nocapture
cargo fmt
```

Expected: target tests pass.

- [ ] **Step 7: Commit**

```bash
git add config/functions.toml config/bindings.toml src/config/type.rs src/device/vision src/func
git commit -m "feat: add dual-line lane detection"
```

---

## Task 5: Inner Circle Detection

**Files:**
- Create: `src/device/vision/inner_circle.rs`
- Modify: `src/device/vision.rs`
- Modify: `src/device/vision/config.rs`
- Modify: `src/config/type.rs`
- Modify: `src/func/functions.rs`
- Modify: `src/func/register.rs`
- Modify: `config/functions.toml`
- Modify: `config/bindings.toml`
- Test: `src/device/vision/tests/inner_circle.rs`

- [ ] **Step 1: Write failing synthetic inner-circle test**

Generate nested circles plus a central letter-like black shape:

```rust
#[test]
fn inner_circle_ignores_letter_and_returns_innermost_ring_center() -> Result<()> {
    let frame = synthetic_nested_circle_with_letter()?;
    let config = InnerCircleDetectConfig::test_px();
    let analysis = analyze_inner_circle_frame(&frame, &config)?;

    assert!(analysis.result.valid);
    assert!(analysis.result.lateral.abs() <= 4);
    assert!(analysis.result.forward.abs() <= 4);
    assert!(analysis.result.score >= 60);
    assert_eq!(
        format_inner_circle_value(&analysis.result).starts_with("INNER_CIRCLE,1,PX"),
        true
    );
    Ok(())
}
```

- [ ] **Step 2: Run RED**

Run:

```bash
cargo test inner_circle_ignores_letter_and_returns_innermost_ring_center -- --nocapture
```

Expected: fails because module does not exist.

- [ ] **Step 3: Implement inner circle analysis**

Implementation outline:

1. grayscale
2. black threshold
3. contour hierarchy with `RETR_TREE`
4. fit ellipses for contour candidates
5. filter by area, aspect ratio, circularity, and ring size
6. group candidates by similar center
7. choose the smallest reliable ring around the nested target, not the central letter
8. convert center to PX/MM
9. annotate selected ellipse and center

Expose:

```rust
pub fn analyze_inner_circle_frame(frame_bgr: &Mat, config: &InnerCircleDetectConfig) -> Result<InnerCircleFrameAnalysis>;
pub fn run_inner_circle_detect_with_frame(config: &InnerCircleDetectConfig) -> Result<InnerCircleDetectOutput>;
```

- [ ] **Step 4: Register function**

Add `InnerCircleDetectParams`, config, validation, `inner_circle_detect`, and registry descriptor.

Default output:

```text
INNER_CIRCLE,valid,unit,lateral,forward,score
```

- [ ] **Step 5: Add ignored GUI test**

Add `show_inner_circle_detect_cv_steps_from_config` to `cv_steps.rs`.

- [ ] **Step 6: Run GREEN**

Run:

```bash
cargo test inner_circle_ignores_letter_and_returns_innermost_ring_center -- --nocapture
cargo test inner_circle_detect -- --nocapture
cargo fmt
```

Expected: target tests pass.

- [ ] **Step 7: Commit**

```bash
git add config/functions.toml config/bindings.toml src/config/type.rs src/device/vision src/func
git commit -m "feat: add inner circle target detection"
```

---

## Task 6: Integration Verification

**Files:**
- Modify only if verification reveals a bug in the previous tasks.

- [ ] **Step 1: Run full verification**

Run:

```bash
cargo fmt --check
cargo test --all-targets
cargo clippy --all-targets
```

Expected:

- `cargo fmt --check` exits 0.
- `cargo test --all-targets` exits 0.
- `cargo clippy --all-targets` exits 0 or only reports the existing `Event` enum postfix warnings already present before this plan.

- [ ] **Step 2: Inspect git state**

Run:

```bash
git status --short
git log --oneline -5
```

Expected: only intentional files are modified or committed. Do not commit the existing user edit in `config/device.toml` unless the user explicitly asks.

- [ ] **Step 3: Final commit if needed**

If integration fixes were required:

```bash
git add <changed-files>
git commit -m "fix: stabilize vision navigation targets"
```

If no fixes were required, no extra commit is needed.

---

## Self-Review Notes

- The plan covers the confirmed in-scope items: ground transform, cross, lane, and inner circle.
- The colored cylinder feature is explicitly deferred.
- Every new function has UART value format, Web image path, automated tests, ignored GUI tests, and config work.
- The `PX/MM/NA` unit field is included in all outputs that can use the ground transform.
