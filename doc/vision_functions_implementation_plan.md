# Remaining Vision Functions Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rebuild QR, BlackRing, and ConcentricRing according to `doc/vision_color_refactor.md` while preserving the existing UART result protocol.

**Architecture:** Each `src/func/*.rs` file owns its strict parameters, single-frame core, Camera-backed multi-frame core, framework adapter, and manually clickable OpenCV tests. Generic OpenCV transforms live in `src/vision/util.rs`; after migration, the monolithic `src/vision/detect.rs` is removed.

**Tech Stack:** Rust 2024, rubo_engine 0.4.0, Tokio, OpenCV 0.92, quircs, serde, serde_json.

### Next Debugging Intent: Ubuntu QR Live Debugging

The Raspberry Pi `rubo-vision.service` is stopped during interactive vision
debugging. VNC is not part of the debugging workflow because its display
latency is too high for camera inspection.

The next step is to use a native Ubuntu desktop with OpenCV GUI support and
run the existing manual QR display test directly:

```bash
cargo test --features opencv test_qr_show -- --nocapture
```

This test remains a manually clickable, long-running test. It uses the
configured `task` camera, displays the original and grayscale frames with
OpenCV, prints detected values as `qr=<value>`, and exits when `q` or `Esc` is
pressed. The normal service must not be started at the same time as this
manual camera test, because both processes would compete for the camera.

Ubuntu-side verification must confirm the camera path, OpenCV GUI display,
QR recognition, and the printed result before the service is started again.

---

### Task 1: Complete Shared Vision Utilities

**Files:**
- Modify: `src/vision/util.rs`

- [ ] **Step 1: Add the reusable grayscale conversion**

```rust
#[cfg(feature = "opencv")]
pub(crate) fn bgr_to_gray(frame: &Mat) -> opencv::Result<Mat> {
    let mut gray = Mat::default();
    imgproc::cvt_color(frame, &mut gray, imgproc::COLOR_BGR2GRAY, 0)?;
    Ok(gray)
}
```

- [ ] **Step 2: Format and verify the default target**

Run: `cargo fmt && cargo check`

Expected: `rubo_vision` compiles without enabling OpenCV.

### Task 2: Rebuild QR

**Files:**
- Modify: `src/func/qr.rs`
- Modify: `config/orangepi/function.toml`
- Modify: `config/raspberrypi/function.toml`
- Modify: `src/config.rs`

- [ ] **Step 1: Replace QR configuration with strict parameters**

```rust
struct QrParameters {
    device_id: String,
    max_frames: usize,
}
```

Read both fields with `ConfigAccess::get`. Reject an empty `device_id` and `max_frames == 0` with `FunctionError::Config`.

- [ ] **Step 2: Implement single-frame and Camera-backed QR cores**

```rust
struct QrFrameResult {
    value: Option<String>,
    gray: opencv::core::Mat,
    frame: opencv::core::Mat,
}

fn analyze_qr_frame(frame: &Mat) -> opencv::Result<QrFrameResult>;

async fn detect_qr(
    camera: Arc<CameraDevice>,
    parameters: &QrParameters,
) -> Result<QrResult, FunctionError>;
```

Decode each grayscale frame with quircs and return the first valid UTF-8 payload. Return `FunctionError::Call` after `max_frames` unsuccessful frames.

- [ ] **Step 3: Keep the framework adapter small**

The adapter reads `QrParameters`, obtains `CameraDevice`, invokes `detect_qr`, stores the raw payload in `value`, and adds JPEG/Base64 `image` only when `call.image_enabled()` is true.

- [ ] **Step 4: Keep only the two approved manual entries**

```rust
#[tokio::test]
async fn test_qr();

#[tokio::test]
async fn test_qr_show();
```

`test_qr` invokes the Camera-backed core. `test_qr_show` invokes the single-frame core and displays original and grayscale frames.

- [ ] **Step 5: Synchronize both profiles and code config**

```toml
[qr_detect]
device_id = "camera"
max_frames = 30
```

Run: `cargo test config::tests::default_rubo_config_test`

Expected: PASS for both profiles.

### Task 3: Rebuild BlackRing

**Files:**
- Modify: `src/func/black_ring.rs`
- Modify: `config/orangepi/function.toml`
- Modify: `config/raspberrypi/function.toml`
- Modify: `src/config.rs`

- [ ] **Step 1: Add strict BlackRing parameter parsing**

```rust
struct BlackRingParameters {
    device_id: String,
    max_frames: usize,
    target_correction: TargetCorrection,
    black_threshold: i32,
    min_radius: f64,
    max_radius: f64,
    min_circularity: f64,
    min_score: u8,
}
```

Validate required frame count, threshold `0..=255`, finite positive radii with `min_radius <= max_radius`, circularity `(0, 1]`, and score `0..=100`.

- [ ] **Step 2: Move the circular-contour algorithm into BlackRing**

```rust
struct BlackRingFrameResult {
    found: bool,
    dx: i32,
    dy: i32,
    score: u8,
    gray: Mat,
    black_mask: Mat,
    frame: Mat,
}

fn analyze_black_ring_frame(
    frame: &Mat,
    parameters: &BlackRingParameters,
) -> opencv::Result<BlackRingFrameResult>;
```

Preserve the approved outer-contour behavior. The Camera-backed core examines `max_frames` and returns the highest score.

- [ ] **Step 3: Build the approved output**

Return `RING,0,0,0,0` when no candidate exists. Successful results use `RING,1,<dx>,<dy>,<score>`. JSON includes `value`, `found`, `dx`, `dy`, and `score`; `image` remains conditional.

- [ ] **Step 4: Implement approved manual entries**

```rust
#[tokio::test]
async fn test_black_ring();

#[tokio::test]
async fn test_black_ring_show();
```

The display entry shows original, grayscale, black mask, and annotated frames.

- [ ] **Step 5: Replace `loop_count` with `max_frames` in both profiles and code config**

Run: `cargo test config::tests::default_rubo_config_test`

Expected: PASS.

### Task 4: Rebuild ConcentricRing

**Files:**
- Modify: `src/func/concentric_ring.rs`
- Modify: `config/orangepi/function.toml`
- Modify: `config/raspberrypi/function.toml`
- Modify: `src/config.rs`

- [ ] **Step 1: Define strict concentric-ring parameters**

```rust
struct ConcentricRingParameters {
    device_id: String,
    max_frames: usize,
    target_correction: TargetCorrection,
    black_threshold: i32,
    close_kernel_size: i32,
    dilate_kernel_size: i32,
    dilate_iterations: i32,
    min_radius: f64,
    max_radius: f64,
    center_tolerance: f64,
    min_arc_points: usize,
    min_ring_score: u8,
}
```

Require positive odd kernels, nonnegative dilation count, valid thresholds/radii/tolerance, at least three arc points, and score `0..=100`.

- [ ] **Step 2: Move only concentric-ring analysis into ConcentricRing**

```rust
struct ConcentricRingFrameResult {
    found: bool,
    dx: i32,
    dy: i32,
    score: u8,
    gray: Mat,
    black_mask: Mat,
    frame: Mat,
}

fn analyze_concentric_ring_frame(
    frame: &Mat,
    parameters: &ConcentricRingParameters,
) -> opencv::Result<ConcentricRingFrameResult>;
```

Remove colored-cylinder candidates, `runtime_param`, message fallback, and colors. Preserve circular-arc fitting and concentric-group scoring.

- [ ] **Step 3: Preserve UART output compatibility**

Return `CROSS,0,0,0,0,0` or `CROSS,0,1,<dx>,<dy>,<score>`. JSON includes `value`, `found`, `dx`, `dy`, and `score`; `image` remains conditional.

- [ ] **Step 4: Implement approved manual entries**

```rust
#[tokio::test]
async fn test_concentric_ring();

#[tokio::test]
async fn test_concentric_ring_show();
```

The display entry shows original, grayscale, final black mask, and annotated frame.

- [ ] **Step 5: Remove ConcentricRing colors and synchronize both profiles/code config**

Run: `cargo test config::tests::default_rubo_config_test`

Expected: PASS.

### Task 5: Remove the Monolithic Vision Module

**Files:**
- Delete: `src/vision/detect.rs`
- Modify: `src/vision/mod.rs`

- [ ] **Step 1: Confirm no references remain**

Run: `rg "vision::detect|ColorDetectConfig|BlackRingDetectConfig|ConcentricRingDetectConfig" src`

Expected: no matches.

- [ ] **Step 2: Delete `detect.rs` and its module export**

Final module file:

```rust
pub(crate) mod util;

#[cfg(test)]
pub mod test;
```

### Task 6: Verify the Refactor

**Files:**
- Verify all modified source/config files.

- [ ] **Step 1: Format and inspect differences**

Run: `cargo fmt`

Run: `git diff --check`

Expected: both succeed.

- [ ] **Step 2: Run default tests and Clippy**

Run: `cargo test`

Run: `cargo clippy --all-targets`

Expected: both succeed without OpenCV installed.

- [ ] **Step 3: Record required Linux/OpenCV verification**

On the Ubuntu/OpenCV target, manually run `test_qr`, `test_qr_show`, `test_black_ring`, `test_black_ring_show`, `test_concentric_ring`, and `test_concentric_ring_show`. No OpenCV installation or download is performed on the current Windows environment.
