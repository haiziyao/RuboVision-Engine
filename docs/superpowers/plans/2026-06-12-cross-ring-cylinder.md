# Cross Ring And Cylinder Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement runtime UART parameters and the `cross` function for concentric black-ring center detection plus color-cylinder centering.

**Architecture:** Extend task events and function runners with one `u8` runtime parameter while keeping typed TOML parameters static. Upgrade UART input to `AA CMD PARAM 55`. Implement `cross` as a device-layer analyzer that first estimates a shared center from multiple circular contours/arcs, then optionally finds the configured color cylinder and reports pixel offset.

**Tech Stack:** Rust 2024, OpenCV 0.75, Tokio, Serde/TOML, existing declarative function registry and message router.

---

## File Structure

- Modify `src/source/traits.rs`: carry `runtime_param` in usual task events.
- Modify `src/source/source_uart.rs`: parse four-byte UART frames and dispatch `CMD` plus `PARAM`.
- Modify `src/source/source_debug.rs`, `src/source/source_timer.rs`, `src/source/source_loop.rs`: create parameterized events, defaulting non-UART sources to zero.
- Modify `src/web/model.rs`, `src/web/handler.rs`: accept optional debug `runtime_param`.
- Modify `src/func/tarits.rs`: pass runtime parameters through `FunctionRunner`, `FunctionWorker`, and typed function builders.
- Modify `src/func/functions.rs`: adapt existing functions and connect `cross` output.
- Modify `src/config/type.rs`, `src/device/vision/config.rs`: define and validate Cross static parameters and color definitions.
- Modify `src/device/vision/camera.rs`: open the Cross camera.
- Replace `src/device/vision/cross.rs`: implement ring/cylinder analysis, multi-frame capture, formatting, and annotation.
- Modify `src/device/vision.rs`: export Cross analysis and output types.
- Create `src/device/vision/tests/cross.rs`: deterministic synthetic OpenCV tests.
- Modify `src/device/vision/tests/mod.rs`, `src/device/vision/tests/support.rs`, `src/device/vision/tests/functional.rs`: include Cross tests and use the new API.
- Modify `config/functions.toml`, `config/bindings.toml`: rename the configured function to `cross`, add static parameters, and enable UART output.
- Modify `README.md`, `TaskTODO.md`: document the four-byte frame and completed Cross behavior.

### Task 1: Carry Runtime Parameters Through Events And Functions

**Files:**
- Modify: `src/source/traits.rs`
- Modify: `src/source/source_debug.rs`
- Modify: `src/source/source_timer.rs`
- Modify: `src/source/source_loop.rs`
- Modify: `src/init/task_dispatch.rs`
- Modify: `src/func/tarits.rs`
- Modify: `src/func/functions.rs`
- Modify: `src/init/task_exec.rs`
- Test: `src/source/source_debug.rs`
- Test: `src/init/task_dispatch.rs`
- Test: `src/init/task_exec.rs`

- [ ] **Step 1: Write failing event and runner tests**

Change usual events from positional data to explicit runtime data:

```rust
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Event {
    UsualEvent {
        task_id: String,
        function_id: String,
        device_id: String,
        runtime_param: u8,
    },
    DebugEvent(String),
    OtherEvent(String),
}
```

Add tests proving:

```rust
#[test]
fn usual_event_keeps_runtime_param() {
    assert_eq!(
        make_event_usual("task", "cross", "camera", 5),
        Event::UsualEvent {
            task_id: "task".into(),
            function_id: "cross".into(),
            device_id: "camera".into(),
            runtime_param: 5,
        }
    );
}
```

```rust
#[test]
fn function_worker_passes_runtime_param_to_runner() -> Result<()> {
    let worker = FunctionWorker::new(
        "test",
        ReturnTargets::default(),
        Arc::new(|runtime_param, _device| {
            Ok(TaskOutput::value("done", runtime_param.to_string()))
        }),
    );

    let result = worker.run(7, &Device::None)?;
    assert_eq!(result.value.as_deref(), Some("7"));
    Ok(())
}
```

- [ ] **Step 2: Run focused tests and verify RED**

Run:

```bash
cargo test runtime_param -- --nocapture
```

Expected: FAIL because `Event`, `make_event_usual`, `FunctionRunner`, and `FunctionWorker::run` do not yet accept runtime parameters.

- [ ] **Step 3: Implement the minimal event and runner changes**

Use:

```rust
pub fn make_event_usual(
    task_id: &str,
    func_id: &str,
    device_id: &str,
    runtime_param: u8,
) -> Event {
    Event::UsualEvent {
        task_id: task_id.to_string(),
        function_id: func_id.to_string(),
        device_id: device_id.to_string(),
        runtime_param,
    }
}
```

Use:

```rust
pub type FunctionRunner =
    Arc<dyn Fn(u8, &Device) -> Result<FunctionResult> + Send + Sync + 'static>;

pub fn run(&self, runtime_param: u8, device: &Device) -> Result<FunctionResult> {
    (self.runner)(runtime_param, device)
}
```

Change typed functions to:

```rust
fn(
    params: &Params,
    runtime_param: u8,
    device: &FunctionDevice,
) -> Result<FunctionResult>
```

and invoke them from the registered runner:

```rust
let runner: FunctionRunner = Arc::new(move |runtime_param, device| {
    let device = FunctionDevice::from_device(device)?;
    function(params.as_ref(), runtime_param, device)
});
```

Add `TaskDispatcher::runtime_param(&Event) -> u8`, and pass the returned value through `TaskListener` to `execute` and `execute_sync`. Existing functions accept `_runtime_param: u8` and ignore it.

Timer, Loop, and DebugSource call `make_event_usual(..., 0)`.

- [ ] **Step 4: Run focused and framework tests**

Run:

```bash
cargo test source::source_debug -- --nocapture
cargo test init:: -- --nocapture
cargo test func:: -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/source src/init src/func
git commit -m "feat: pass runtime parameters to functions"
```

### Task 2: Upgrade UART And Web Debug Trigger Parameters

**Files:**
- Modify: `src/source/source_uart.rs`
- Modify: `src/web/model.rs`
- Modify: `src/web/handler.rs`
- Modify: `README.md`
- Test: `src/source/source_uart.rs`
- Test: `src/web/handler.rs`

- [ ] **Step 1: Write failing UART parser tests**

Replace command-only expectations with:

```rust
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct UartFrame {
    command: u8,
    param: u8,
}
```

Add tests:

```rust
#[test]
fn takes_complete_four_byte_frame() {
    let mut pending = vec![0xAA, 0x03, 0x05, 0x55];
    assert_eq!(
        take_uart_frames(&mut pending),
        vec![UartFrame {
            command: 0x03,
            param: 0x05,
        }]
    );
    assert!(pending.is_empty());
}
```

```rust
#[test]
fn keeps_partial_frame_until_tail_arrives() {
    let mut pending = vec![0xAA, 0x03, 0x02];
    assert!(take_uart_frames(&mut pending).is_empty());
    pending.push(0x55);
    assert_eq!(take_uart_frames(&mut pending)[0].param, 0x02);
}
```

```rust
#[test]
fn old_three_byte_frame_does_not_dispatch() {
    let mut pending = vec![0xAA, 0x03, 0x55];
    assert!(take_uart_frames(&mut pending).is_empty());
    assert_eq!(pending, vec![0xAA, 0x03, 0x55]);
}
```

- [ ] **Step 2: Run UART tests and verify RED**

Run:

```bash
cargo test source::source_uart::tests -- --nocapture
```

Expected: FAIL because the parser still consumes three-byte frames and returns only commands.

- [ ] **Step 3: Implement the four-byte parser**

Set:

```rust
const UART_FRAME_LEN: usize = 4;
```

Parse:

```rust
if pending[3] != UART_FRAME_TAIL {
    pending.drain(..1);
    continue;
}

frames.push(UartFrame {
    command: pending[1],
    param: pending[2],
});
pending.drain(..UART_FRAME_LEN);
```

Dispatch task commands with:

```rust
let event = make_event_usual(
    bind.task_id.as_str(),
    bind.function_id.as_str(),
    bind.device_id.as_str(),
    frame.param,
);
```

Reserved stop/status commands ignore `frame.param`.

- [ ] **Step 4: Add failing Web debug parameter test**

Extend the request:

```rust
pub struct DebugTriggerRequest {
    pub source_key: String,
    #[serde(default)]
    pub runtime_param: u8,
}
```

Test that a request with `runtime_param: 4` produces an event whose parameter is `4`, while an omitted JSON field defaults to `0`.

- [ ] **Step 5: Implement DebugSource parameter forwarding**

Change:

```rust
pub async fn trigger(
    &self,
    source_key: &str,
    runtime_param: u8,
) -> Result<Event, DebugSourceError>
```

The handler passes `request.runtime_param`; existing direct callers use zero.

- [ ] **Step 6: Run UART and Web tests**

Run:

```bash
cargo test source::source_uart -- --nocapture
cargo test web::handler -- --nocapture
cargo test source::source_debug -- --nocapture
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add src/source/source_uart.rs src/source/source_debug.rs src/web README.md
git commit -m "feat: parse runtime params from uart frames"
```

### Task 3: Add Typed Cross Configuration And Registration

**Files:**
- Modify: `src/config/type.rs`
- Modify: `src/device/vision/config.rs`
- Modify: `src/func/functions.rs`
- Modify: `src/func/register.rs`
- Modify: `src/device/vision.rs`
- Modify: `src/device/vision/tests/support.rs`
- Modify: `src/device/vision/tests/config.rs`
- Modify: `config/functions.toml`
- Modify: `config/bindings.toml`
- Test: `src/func/functions.rs`
- Test: `src/func/register.rs`
- Test: `src/device/vision/tests/config.rs`

- [ ] **Step 1: Write failing Cross config tests**

Define the desired typed API in tests:

```rust
#[test]
fn cross_config_loads_target_and_five_colors() -> Result<()> {
    let config = cross_config_from_config()?;
    assert_eq!(config.target_correction.x, 0);
    assert_eq!(config.target_correction.y, 0);
    assert_eq!(config.colors.len(), 5);
    assert_eq!(config.colors[0].id, 1);
    Ok(())
}
```

Add validation tests for duplicate IDs, missing ID `5`, invalid HSV, invalid radius order, and out-of-range `min_ring_score`.

- [ ] **Step 2: Run config tests and verify RED**

Run:

```bash
cargo test cross_config -- --nocapture
cargo test register_func_rejects_invalid_cross -- --nocapture
```

Expected: FAIL because `CrossDetectParams` is empty and the configured function is still `cross_detect`.

- [ ] **Step 3: Add typed configuration**

Use:

```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CrossColorConfig {
    pub id: u8,
    pub name: String,
    pub hsv: [i32; 6],
    pub min_area: f64,
    pub min_circularity: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CrossDetectParams {
    pub debug_model: bool,
    pub loop_count: i32,
    pub target_correction: TargetCorrectionConfig,
    pub black_threshold: i32,
    pub min_radius: f64,
    pub max_radius: f64,
    pub center_tolerance: f64,
    pub min_arc_points: usize,
    pub min_ring_score: u8,
    pub colors: Vec<CrossColorConfig>,
}
```

Mirror these fields in `CrossDetectConfig` and convert the color entries in `from_params`.

Validation enforces:

```rust
let ids: HashSet<u8> = self.colors.iter().map(|color| color.id).collect();
if ids != HashSet::from([1, 2, 3, 4, 5]) {
    return Err(anyhow!("cross colors must contain unique ids 1 through 5"));
}
```

Reuse the existing HSV range validation rules from color detection.

- [ ] **Step 4: Rename the registered function and configuration**

Change the declaration to:

```rust
cross(params: CrossDetectParams, device: CameraDevice) => cross,
```

Use `function_id = "cross"` in functions and bindings TOML. Enable:

```toml
returns = { web = true, uart = true }
```

Populate the confirmed Cross parameters and five configurable colors.

- [ ] **Step 5: Run config and registry tests**

Run:

```bash
cargo test config -- --nocapture
cargo test func::register -- --nocapture
cargo test device::vision::tests::config -- --nocapture
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/config src/device/vision/config.rs src/device/vision/tests src/func config
git commit -m "feat: configure cross detection modes"
```

### Task 4: Detect A Shared Center From Concentric Rings

**Files:**
- Replace: `src/device/vision/cross.rs`
- Modify: `src/device/vision.rs`
- Create: `src/device/vision/tests/cross.rs`
- Modify: `src/device/vision/tests/mod.rs`
- Test: `src/device/vision/tests/cross.rs`

- [ ] **Step 1: Write a failing complete-ring test**

Create a synthetic frame with four rings centered at `(350, 220)`:

```rust
#[test]
fn cross_zero_finds_concentric_ring_center_and_target_offset() -> Result<()> {
    let frame = synthetic_ring_frame(Point::new(350, 220), &[45, 80, 120, 165])?;
    let config = base_cross_config();
    let analysis = analyze_cross_frame(&frame, 0, &config)?;

    assert!(analysis.result.valid);
    let center = analysis.result.ring_center.expect("ring center");
    assert!((center.x - 350.0).abs() <= 5.0);
    assert!((center.y - 220.0).abs() <= 5.0);
    assert_eq!(analysis.result.dx, 30);
    assert_eq!(analysis.result.dy, -20);
    Ok(())
}
```

- [ ] **Step 2: Run the test and verify RED**

Run:

```bash
cargo test cross_zero_finds_concentric_ring_center_and_target_offset -- --nocapture
```

Expected: FAIL because `analyze_cross_frame` and Cross result types do not exist.

- [ ] **Step 3: Implement Cross result types and preprocessing**

Add:

```rust
#[derive(Debug, Clone)]
pub struct CrossResult {
    pub param: u8,
    pub valid: bool,
    pub ring_center: Option<Point2f>,
    pub cylinder_center: Option<Point2f>,
    pub dx: i32,
    pub dy: i32,
    pub score: u8,
}

pub struct CrossFrameAnalysis {
    pub result: CrossResult,
    pub gray: Mat,
    pub black_mask: Mat,
    pub annotated: Mat,
}
```

Implement `analyze_cross_frame(frame_bgr, runtime_param, config)` with grayscale, Gaussian blur, inverse threshold, and a small elliptical open/close kernel.

- [ ] **Step 4: Implement circular contour candidates**

For each contour with at least `min_arc_points`:

1. Fit `min_enclosing_circle`.
2. Reject radius outside the configured range.
3. Compute each point's radial residual from the fitted radius.
4. Compute angular coverage by sorting point angles and subtracting the largest gap.
5. Score residual, coverage, and point count.

Use:

```rust
struct RingCandidate {
    center: Point2f,
    radius: f32,
    coverage: f64,
    residual: f64,
    weight: f64,
}
```

Group candidates whose centers are within `center_tolerance`; reward multiple distinct radius bands and use weighted center averaging.

- [ ] **Step 5: Run the complete-ring test and verify GREEN**

Run:

```bash
cargo test cross_zero_finds_concentric_ring_center_and_target_offset -- --nocapture
```

Expected: PASS.

- [ ] **Step 6: Write failing partial-occlusion and invalid-scene tests**

Add:

```rust
#[test]
fn cross_zero_uses_visible_arcs_when_center_is_occluded() -> Result<()> {
    let mut frame = synthetic_ring_frame(Point::new(300, 250), &[50, 90, 135, 180])?;
    imgproc::rectangle(
        &mut frame,
        Rect::new(260, 210, 180, 170),
        Scalar::new(230.0, 230.0, 230.0, 0.0),
        -1,
        imgproc::LINE_8,
        0,
    )?;
    let result = analyze_cross_frame(&frame, 0, &base_cross_config())?.result;
    assert!(result.valid);
    assert!((result.ring_center.unwrap().x - 300.0).abs() <= 7.0);
    Ok(())
}
```

Add a blank/noise frame test that requires `valid == false`.

- [ ] **Step 7: Refine candidate grouping until both tests pass**

Split contours into edge chains when needed, retain incomplete contours, and require at least two distinct supported radii for a valid shared center. Keep the score below `min_ring_score` for unsupported single circles.

Run:

```bash
cargo test device::vision::tests::cross -- --nocapture
```

Expected: PASS.

- [ ] **Step 8: Add formatting and annotation tests**

Test:

```rust
assert_eq!(
    format_cross_value(&CrossResult {
        param: 0,
        valid: true,
        ring_center: None,
        cylinder_center: None,
        dx: -42,
        dy: 18,
        score: 91,
    }),
    "CROSS,0,1,-42,18,91"
);
```

Invalid results format as `CROSS,param,0,0,0,0`. Assert the annotated image is non-empty.

- [ ] **Step 9: Commit**

```bash
git add src/device/vision/cross.rs src/device/vision.rs src/device/vision/tests
git commit -m "feat: detect concentric ring center"
```

### Task 5: Detect Configured Color Cylinders

**Files:**
- Modify: `src/device/vision/cross.rs`
- Modify: `src/device/vision/tests/cross.rs`
- Test: `src/device/vision/tests/cross.rs`

- [ ] **Step 1: Write failing colored-cylinder offset test**

Draw a red filled circle centered at `(390, 260)` over rings centered at `(320, 240)`:

```rust
#[test]
fn cross_one_reports_red_cylinder_offset_from_ring_center() -> Result<()> {
    let frame = synthetic_ring_with_cylinder(
        Point::new(320, 240),
        Point::new(390, 260),
        Scalar::new(0.0, 0.0, 255.0, 0.0),
    )?;
    let result = analyze_cross_frame(&frame, 1, &base_cross_config())?.result;

    assert!(result.valid);
    assert_eq!(result.dx, 70);
    assert_eq!(result.dy, 20);
    assert!(result.cylinder_center.is_some());
    Ok(())
}
```

- [ ] **Step 2: Run the test and verify RED**

Run:

```bash
cargo test cross_one_reports_red_cylinder_offset_from_ring_center -- --nocapture
```

Expected: FAIL because cylinder detection is not implemented.

- [ ] **Step 3: Implement HSV and geometric cylinder candidates**

Find the selected color by `runtime_param`, create HSV mask, apply close/open morphology, and score external contours by:

- area >= `min_area`
- circularity >= `min_circularity`
- bounding-box aspect in a conservative circular range
- solid fill ratio
- center within the detected ring region
- proximity to `ring_center`

Use the contour moments for cylinder center and set:

```rust
dx = cylinder_center.x.round() as i32 - ring_center.x.round() as i32;
dy = cylinder_center.y.round() as i32 - ring_center.y.round() as i32;
```

- [ ] **Step 4: Run the red-cylinder test and verify GREEN**

Run:

```bash
cargo test cross_one_reports_red_cylinder_offset_from_ring_center -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Add failing wrong-color and black-cylinder tests**

Add one test where parameter `2` is requested but only a red cylinder exists and require `valid == false`.

Add one test with a large black filled circle over thin black rings and require the detected cylinder center to match the filled circle rather than a ring contour.

- [ ] **Step 6: Implement black/white discrimination and pass all Cross tests**

For black targets, require high contour fill ratio and minimum area large enough to reject thin ring bands. For white targets, use low saturation, high value, geometric score, and ring-region restriction.

Run:

```bash
cargo test device::vision::tests::cross -- --nocapture
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add src/device/vision/cross.rs src/device/vision/tests/cross.rs
git commit -m "feat: detect color cylinder offsets"
```

### Task 6: Add Camera Execution And Function Output

**Files:**
- Modify: `src/device/vision/camera.rs`
- Modify: `src/device/vision/cross.rs`
- Modify: `src/device/vision.rs`
- Modify: `src/func/functions.rs`
- Modify: `src/device/vision/tests/functional.rs`
- Modify: `src/device/vision/tests/manual_web.rs`
- Modify: `TaskTODO.md`
- Test: `src/func/functions.rs`

- [ ] **Step 1: Write failing function-output test**

Add:

```rust
#[test]
fn cross_output_to_function_result_keeps_uart_value_and_image() -> Result<()> {
    let frame = Mat::new_rows_cols_with_default(
        8,
        8,
        core::CV_8UC3,
        core::Scalar::all(255.0),
    )?;
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

    assert_eq!(result.value.as_deref(), Some("CROSS,1,1,20,-8,88"));
    assert!(result.image.as_deref().is_some_and(|value| {
        value.starts_with("data:image/jpeg;base64,")
    }));
    Ok(())
}
```

- [ ] **Step 2: Run the output test and verify RED**

Run:

```bash
cargo test cross_output_to_function_result_keeps_uart_value_and_image -- --nocapture
```

Expected: FAIL because Cross output is still a plain string without a frame.

- [ ] **Step 3: Implement Cross camera execution**

Add `register_cross_camera(config)` in `camera.rs`.

Implement:

```rust
pub fn run_cross_detect_with_frame(
    runtime_param: u8,
    config: &CrossDetectConfig,
) -> Result<CrossDetectOutput>
```

Validate `runtime_param` before opening the camera, read `loop_count` frames, analyze each, optionally display debug windows, and retain the highest score.

- [ ] **Step 4: Connect the function layer**

Implement:

```rust
fn cross(
    params: &CrossDetectParams,
    runtime_param: u8,
    camera: &CameraDevice,
) -> Result<FunctionResult> {
    let config = CrossDetectConfig::from_params(params, camera);
    cross_output_to_function_result(run_cross_detect_with_frame(
        runtime_param,
        &config,
    )?)
}
```

Convert the annotated Mat to JPEG data URL and use `TaskOutput::value_with_image`.

- [ ] **Step 5: Update functional/manual tests and task status**

Pass runtime parameter `0` in camera tests. Update the manual Web loop so its title and message show the selected parameter. Mark the real Cross implementation complete in `TaskTODO.md`.

- [ ] **Step 6: Run Cross and function tests**

Run:

```bash
cargo test cross -- --nocapture
cargo test func::functions::tests -- --nocapture
```

Expected: PASS, with hardware-only tests still ignored.

- [ ] **Step 7: Commit**

```bash
git add src/device/vision src/func/functions.rs TaskTODO.md
git commit -m "feat: expose cross detection results"
```

### Task 7: Full Regression And Documentation Verification

**Files:**
- Modify: `README.md`
- Modify: any files needed to resolve verified regressions

- [ ] **Step 1: Update protocol and Cross documentation**

Document:

```text
AA CMD PARAM 55
```

Include the Cross parameter table, coordinate signs, result format, default `PARAM=0` for other functions, and note that old three-byte input is unsupported.

- [ ] **Step 2: Format and inspect changes**

Run:

```bash
cargo fmt
git diff --check
git status --short
```

Expected: no formatting or whitespace errors; only intended files changed.

- [ ] **Step 3: Run the complete verification suite**

Run:

```bash
cargo fmt --check
cargo check
cargo test --all-targets
```

Expected: all commands exit zero; ignored camera/GUI tests remain ignored.

- [ ] **Step 4: Review requirements against the design**

Verify explicitly:

- `function_id` is always `cross`.
- UART passes `PARAM` independently.
- `0` uses the corrected screen center.
- `1..5` use the detected ring center.
- partial circular arcs support center recovery.
- invalid visual evidence returns `valid=0`.
- output reaches UART and Web with an annotated image.

- [ ] **Step 5: Commit**

```bash
git add README.md
git add -u
git commit -m "docs: document cross runtime parameters"
```
