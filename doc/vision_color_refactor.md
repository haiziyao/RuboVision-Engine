# Vision and Color Refactor Specification

## 1. Document purpose

This document records the approved direction for rebuilding the Vision code and the Color function in the RuboVision example project.

It has three purposes:

1. Preserve the useful OpenCV encapsulation style from the original `main` branch.
2. Define the required organization of a framework Function without mixing reusable OpenCV operations, business logic, framework adaptation, and interactive debugging.
3. Prevent later implementation from silently inventing file names, type names, fields, default values, or behavior that has not been approved.

Color was the first implementation target. QR, BlackRing, and Cross designs were subsequently
reviewed and approved in Sections 14 through 16.

## 2. Scope

The complete Vision rebuild eventually covers:

- Camera frame acquisition.
- Reusable OpenCV helper functions.
- Color detection.
- QR detection.
- Black-ring detection.
- Cross and colored-object detection.
- Framework Function adapters.
- Runtime result construction.
- Hardware tests and interactive OpenCV debugging tools.

The first implementation phase covers only:

- Reusable OpenCV helpers required by Color.
- Color single-frame analysis.
- Color multi-frame confirmation.
- Color framework adaptation.
- Color tests, real-time visualization, and HSV tuning.

## 3. Problems in the current implementation

### 3.1 A monolithic Vision implementation

The current `src/vision/detect.rs` contains approximately 640 lines and combines:

- Output types.
- Color configuration types.
- Ring configuration types.
- Cross configuration types.
- Color detection.
- QR decoding.
- Black-ring detection.
- Cross detection.
- Generic OpenCV transformations.
- Drawing helpers.

These responsibilities are unrelated enough that changing one function risks affecting the others. Reusable operations such as BGR-to-gray conversion and HSV masking are hidden inside a business-algorithm file.

### 3.2 Function adapters do too much

Each current visual Function performs most of the following work:

1. Read configuration.
2. Find the Camera Device.
3. Read multiple frames.
4. Build an algorithm-specific configuration object.
5. Start a blocking Tokio task.
6. Execute the OpenCV algorithm.
7. Encode the result frame as JPEG.
8. Encode JPEG bytes as Base64.
9. Build the JSON `FuncResult`.
10. Convert several unrelated errors into `FunctionError`.

This makes the Function adapter difficult to read and creates almost identical code in Color, QR, black-ring, and Cross.

### 3.3 Tests do not follow one implementation path

The current normal tests call the framework `Function::call` implementation, while the real-time `imshow` tests call private `*_output` functions directly.

As a result:

- The normal test verifies framework adaptation but cannot inspect intermediate OpenCV images.
- The display test inspects OpenCV output but bypasses framework adaptation and result construction.
- The two tests can behave differently even though they claim to test the same feature.

### 3.4 Multi-frame behavior was changed incorrectly

The original Color implementation confirmed a result only after repeatedly detecting the same color. The current implementation collects a fixed set of frames and selects the frame with the highest color ratio.

These are not equivalent:

- Selecting the best frame can accept one accidental detection.
- Consecutive confirmation rejects unstable or transient detections.
- Reading every frame before processing prevents early success.

The rebuilt Color function must restore explicit consecutive-frame confirmation.

### 3.5 Configuration currently hides mistakes

The current visual Functions frequently use `get_or`, for example:

```rust
let loop_count = config.get_or("loop_count", 30_i32)?;
```

This silently replaces a missing configuration value. The project requirement is strict:

- Every required parameter must exist.
- Every required parameter must have the correct type.
- Invalid or missing parameters must produce the prescribed error.
- The function must never continue with a default value.

## 4. Useful design from the original main branch

The original `main` branch contains `src/utils/cv_util.rs`. Its implementation quality is not accepted wholesale, but its responsibility boundary is useful.

It encapsulates small reusable OpenCV operations such as:

- `bgr_to_gray`
- HSV bound construction
- HSV `inRange`
- Binary thresholding
- Morphological opening and closing
- Box and Gaussian blur
- Circular ROI and mask construction

The key principle is:

> OpenCV operations that are independent of Color, QR, rings, Cross, Camera registration, and framework configuration belong in a reusable Vision utility layer.

The utility layer must not:

- Read `FuncConfig`.
- Open a Camera.
- Decide whether a color is accepted.
- Count matching frames.
- Build `FuncResult`.
- Know about WebSink or UartSink.
- Display windows unless the helper is explicitly a generic display helper approved for that purpose.

## 5. Approved architecture

### 5.1 Layer 1: reusable Vision utilities

This layer wraps repetitive OpenCV API calls and returns OpenCV data.

Example responsibility:

```rust
// Conceptual example; the implementation uses the names recorded below.
fn make_hsv_mask(frame: &Mat, hsv: [i32; 6]) -> opencv::Result<Mat>;
fn make_circle_roi(frame: &Mat, radius_ratio: f64) -> opencv::Result<(Mat, Mat)>;
fn masked_ratio(mask: &Mat, roi_mask: &Mat) -> opencv::Result<f64>;
```

This layer performs transformations, not business decisions.

### 5.2 Layer 2: Color single-frame analysis

The single-frame Color function receives one frame and explicitly supplied Color parameters.

It must:

1. Build the circular ROI and ROI mask.
2. Convert the source frame to HSV only as often as necessary.
3. Build a mask for each configured color.
4. Restrict each color mask to the ROI.
5. Calculate the occupied ratio for every color.
6. Select the strongest candidate.
7. Apply the configured minimum ratio.
8. Produce the candidate result and the important intermediate images required by debugging.

It must not:

- Read from Camera.
- Read framework configuration directly.
- Count frames.
- Construct `FuncResult`.
- Encode JPEG/Base64.
- Send data to any Sink.

Conceptual example:

```rust
// Conceptual example; the implementation uses the names recorded below.
let frame_result = analyze_color_frame(&frame, &parameters)?;

println!("candidate={}", frame_result.color);
println!("ratio={}", frame_result.ratio);
```

### 5.3 Layer 3: Color multi-frame confirmation

The multi-frame Color function receives Camera access and explicit parameters. It repeatedly obtains a frame and calls the single-frame analyzer.

Approved behavior:

1. Keep a current candidate color.
2. Keep the number of consecutive frames matching that candidate.
3. If the current frame is `unknown`, clear the candidate and reset the count.
4. If the current frame detects a different known color, replace the candidate and set the count to one.
5. If the current frame detects the same candidate, increment the count.
6. Return success when the consecutive count reaches the configured confirmation count.
7. Return the prescribed detection error when the configured maximum frame count is reached.
8. Stop reading immediately after success; do not capture all frames first.

Conceptual flow:

```text
Camera frame
    -> single-frame Color analysis
    -> unknown: reset
    -> new color: candidate=color, count=1
    -> same color: count=count+1
    -> count reaches requirement: success
    -> maximum frame count reached: error
```

This layer is the common Color core used by both the framework Function and the normal Color test.

### 5.4 Layer 4: framework Function adapter

The framework-facing Color implementation must remain short and obvious.

It is responsible only for:

1. Strictly reading every required configuration field.
2. Obtaining the configured Camera from `FunctionDevices`.
3. Calling the multi-frame Color core.
4. Converting the approved Color result into `FuncResult`.
5. Propagating configuration, Camera, OpenCV, task, and detection errors through the framework error path.

Conceptual example:

```rust
#[async_trait]
impl Function for ColorFunction {
    async fn call(&self, call: FunctionCall<'_>) -> Result<FuncResult, FunctionError> {
        let parameters = read_color_parameters(call.function_config())?;
        let camera = call.devices().get::<CameraDevice>(parameters.device_id())?;
        let result = detect_color(camera, &parameters).await?;
        build_color_func_result(result)
    }
}
```

The example demonstrates responsibility only. Actual names are defined by the implementation
recorded in this document.

## 6. Required organization of the Color file

The Color implementation must contain four clearly identifiable sections.

### 6.1 Core feature section

Contains:

- Strict Color parameter representation or parameter access.
- Single-frame analysis.
- Multi-frame confirmation.
- Color-specific result construction.
- Color-specific drawing required for the final annotated result.

### 6.2 Framework section

Contains:

- Framework registration macro.
- The framework Function type.
- `Function::call` implementation.
- Conversion from the Color result to `FuncResult`.

### 6.3 Test section

Contains a manually clickable test that:

1. Loads the real application configuration.
2. Loads the active platform profile.
3. Registers and creates the configured Camera automatically.
4. Calls the Color core directly.
5. Prints or asserts the approved result.

The test must not require the developer to manually construct a partial `FuncConfig` or manually edit the test before running it.

### 6.4 Interactive debugging section

Contains manually clickable tools for:

- Continuous Color detection.
- Continuous display of important intermediate images.
- HSV range tuning.

These tools are development operations, not runtime behavior. Production Function execution must never call `imshow` or `wait_key`.

## 7. Required Color tests and debugging tools

Names remain subject to approval, but three independent clickable entries are required.

### 7.1 Normal Color test

Purpose:

- Automatically inject the complete configuration.
- Automatically register and create Camera.
- Call the same multi-frame Color core used by the framework adapter.
- Verify the final detection result.

It must not call WebSink or UartSink.

### 7.2 Real-time Color display

Purpose:

- Continuously obtain Camera frames.
- Execute the real single-frame Color analysis.
- Display the intermediate images and final result.
- Exit cleanly when `q` or `Esc` is pressed.

The display should include as many useful stages as practical:

1. Original frame.
2. Circular ROI.
3. Mask for each configured color.
4. Selected candidate mask.
5. Final annotated result.

No separate simplified detection algorithm may be written for this tool. It must consume the output of the actual single-frame analyzer.

### 7.3 HSV tuning tool

Purpose:

- Open Camera using the active configuration.
- Provide H-min, H-max, S-min, S-max, V-min, and V-max controls.
- Apply the current HSV range to every incoming frame.
- Restrict the mask to the configured circular ROI.
- Display original, ROI, mask, and masked result images.
- Allow the final six values to be read clearly for transfer into the configuration file.

The original `main` branch `test_hsv` is the behavioral reference, but its code should be cleaned rather than copied directly.

## 8. Color configuration requirements

The accepted Color configuration is:

```toml
device_id = "camera"
max_frames = 30
confirm_frames = 5
radius_ratio = 0.4
min_area_ratio = 0.8

[[colors]]
name = "red"
hsv_ranges = [
    [0, 10, 160, 255, 110, 255],
    [170, 179, 160, 255, 110, 255],
]
```

One logical color owns one or more HSV ranges. This supports colors such as red whose Hue values
cross the `0/179` boundary. The masks for all ranges belonging to one color are merged before its
occupied-area ratio is calculated.

The obsolete `debug_model` field should not control production behavior. Interactive debugging belongs to explicit test/debug functions.

The old `loop_count` does not express whether it means maximum attempts, batch size, or required matches and therefore must not be retained without an explicit decision.

## 9. Strict configuration rules

No required visual parameter may use a default fallback.

Forbidden pattern:

```rust
let count = config.get_or("max_frames", 30)?;
```

Required pattern:

```rust
let count = config.get("max_frames")?;
```

In addition to type parsing, semantic validation is required. The final accepted rules must include decisions for:

- Maximum frame count greater than zero.
- Confirmation count greater than zero.
- Confirmation count not greater than maximum frame count.
- ROI ratio finite and greater than zero. Values above `0.5` are permitted and produce a circle
  clipped by the image boundary.
- Minimum area ratio within a valid numeric range.
- Non-empty color list.
- Non-empty logical color names and at least one HSV range per color.
- HSV H values within OpenCV's accepted range.
- HSV S and V values within their accepted ranges.
- Minimum values not greater than maximum values.

An invalid configuration must return the prescribed error and stop that Function execution. It must never be corrected silently.

## 10. Runtime result requirements

The runtime result supports two different consumers:

- UART requires a small control value, normally the detected color name.
- Web debugging may require text, confidence/ratio information, and an annotated image.

The implemented result is:

```json
{
  "text": "color_detect finished: red",
  "value": "red",
  "ratio": 0.91,
  "image": "data:image/jpeg;base64,..."
}
```

`image` is omitted when `FunctionCall::image_enabled()` is false.

## 11. Web output and competition performance

Before RuboEngine `0.4.0`, the application always encoded the annotated `Mat` as JPEG and Base64
inside the Function before Sink routing started.

Therefore removing WebSink currently saves:

- Web history insertion.
- Output cloning for Web frames.
- SSE serialization.
- Browser network traffic.
- Some memory use.

It does not save:

- JPEG encoding.
- Base64 encoding.
- Construction of the large image string.

The output decision is provided by RuboEngine before encoding through
`FunctionCall::image_enabled()`.

Important constraint:

> Engine cannot recover the JPEG/Base64 cost by deleting the `image` field after the Function returns. The decision must be available before image generation, or media generation must be lazy.

RuboEngine `0.4.0` reads `web.output_image` from application config. Image output is enabled only
when Web is enabled, `output_image` is true, and the active Binding routes to the `web` Sink.

## 12. Shared requirements for the remaining visual functions

QR, BlackRing, and Cross follow the same high-level organization after separate approval:

1. Reusable OpenCV operations in the Vision utility layer.
2. A single-frame core independent of Camera and framework configuration.
3. A Camera/multi-frame core when the feature needs repeated observations.
4. A small framework adapter.
5. A normal test with automatic configuration and Device injection.
6. A real-time debugging entry displaying important processing stages.
7. Additional parameter-tuning tools when the algorithm needs them.
8. Strict required configuration with no defaults.

Color's parameters, result fields, and confirmation rules are not copied into the other functions.
Their approved differences are recorded in Sections 14 through 16.

## 13. Implemented first phase

- Reusable OpenCV operations are in `src/vision/util.rs`.
- Color core, framework adapter, and interactive entries are in `src/func/color.rs`.
- The old Color implementation has been removed from `src/vision/detect.rs`.
- Required configuration uses strict `get(...)` access and semantic validation.
- Multi-frame confirmation stops immediately after the configured consecutive count succeeds.
- `test_color` exercises the Camera-backed Color core.
- `test_color_show` displays original, ROI, every color mask, selected mask, and annotated result.
- `test_hsv` provides six HSV trackbars and displays original, ROI, mask, and masked result.

## 14. Approved QR refactor

QR remains in `src/func/qr.rs`. Its configuration contains only:

```toml
[qr_detect]
device_id = "camera"
max_frames = 30
```

Both fields are required. `device_id` must not be empty and `max_frames` must be greater than zero.
Each frame is converted to grayscale and decoded. The first valid UTF-8 payload succeeds
immediately. QR does not use consecutive-frame confirmation because QR decoding already validates
the encoded data. Failure to decode within `max_frames` returns `FunctionError::Call`.

The runtime `value` is the unmodified QR UTF-8 payload so the existing UART protocol remains
unchanged. The result image is generated only when `FunctionCall::image_enabled()` is true.

- `test_qr` loads active configuration and Camera, then calls the Camera-backed QR core.
- `test_qr_show` continuously displays the original and grayscale frames and prints decoded data.

`debug_model` and `loop_count` are removed.

## 15. Approved BlackRing refactor

BlackRing remains in `src/func/black_ring.rs`. Its required configuration is:

```toml
[black_ring_detect]
device_id = "camera"
max_frames = 3
black_threshold = 90
min_radius = 20.0
max_radius = 180.0
min_circularity = 0.65
min_score = 50

[black_ring_detect.target_correction]
x = 0
y = 0
```

The single-frame core produces the grayscale frame, black mask, best circular candidate, and
annotated frame. The Camera-backed core examines `max_frames` and returns the highest-scoring
candidate. This phase deliberately preserves the current circular outer-contour algorithm; it does
not add inner-hole validation.

No candidate is a normal business result rather than a framework error:

```text
RING,0,0,0,0
```

A successful result remains:

```text
RING,1,<dx>,<dy>,<score>
```

The runtime JSON contains `value`, `found`, `dx`, `dy`, and `score`. `image` is conditional on
`FunctionCall::image_enabled()`.

- `test_black_ring` loads configuration and Camera and calls the complete core.
- `test_black_ring_show` displays original, grayscale, black mask, and annotated frames.

`debug_model` and `loop_count` are removed. Numeric configuration is validated for meaningful
threshold, radius, circularity, score, and frame-count ranges.

## 16. Approved Cross refactor

Cross remains in `src/func/cross.rs`. It only locates the center of a group of concentric circular
arcs. The unreachable colored-cylinder mode, `runtime_param` parsing, and `colors` configuration are
removed.

Required configuration:

```toml
[cross]
device_id = "camera"
max_frames = 3
black_threshold = 90
close_kernel_size = 5
dilate_kernel_size = 3
dilate_iterations = 1
min_radius = 20.0
max_radius = 600.0
center_tolerance = 14.0
min_arc_points = 24
min_ring_score = 50

[cross.target_correction]
x = 0
y = 0
```

The single-frame flow is grayscale, black threshold, closing/dilation, circular-arc fitting,
concentric-group scoring, and center-offset calculation. The Camera-backed core examines
`max_frames` and returns the highest-scoring frame.

The UART-compatible values remain:

```text
CROSS,0,0,0,0,0
CROSS,0,1,<dx>,<dy>,<score>
```

The fixed `0` is retained only as a protocol field; it is no longer read from Message payload or
used to select an algorithm. The runtime JSON contains `value`, `found`, `dx`, `dy`, and `score`.
`image` is conditional on `FunctionCall::image_enabled()`.

- `test_cross` loads configuration and Camera and calls the complete core.
- `test_cross_show` displays original, grayscale, black mask, and annotated frames.

`debug_model`, `loop_count`, colors, and payload fallback behavior are removed. Kernel sizes must be
positive odd numbers; frame counts, thresholds, radii, tolerances, arc counts, and scores receive
strict semantic validation.

## 17. Final Vision file layout

After all three migrations:

```text
src/vision/
  mod.rs
  util.rs
  test.rs

src/func/
  color.rs
  qr.rs
  black_ring.rs
  cross.rs
  debug.rs
```

`src/vision/detect.rs` is deleted because its QR, BlackRing, and Cross responsibilities move into
their respective Function files. Reusable OpenCV operations move to `src/vision/util.rs`; business
algorithms do not move there.
