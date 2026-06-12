# Cross Mask Morphology Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stabilize Cross ring detection by reconnecting short black-arc gaps and thickening the thresholded strokes before contour fitting.

**Architecture:** Extend typed Cross configuration with bounded morphology controls. Keep grayscale conversion, thresholding, contour fitting, grouping, and result formatting unchanged; only replace the final mask cleanup with elliptical closing followed by optional dilation.

**Tech Stack:** Rust 2024, OpenCV morphology APIs, Serde/TOML typed configuration, Cargo tests.

---

### Task 1: Add Typed Morphology Configuration

**Files:**
- Modify: `src/config/type.rs`
- Modify: `src/device/vision/config.rs`
- Modify: `src/func/functions.rs`
- Modify: `src/func/register.rs`
- Modify: `config/functions.toml`
- Modify: `src/device/vision/tests/cross.rs`

- [ ] **Step 1: Write failing validation tests**

Add registry tests that set `close_kernel_size = 4` and
`dilate_iterations = 6`, then assert `register_func(functions).is_err()`.

- [ ] **Step 2: Run tests and verify RED**

Run:

```bash
cargo test register_func_rejects_invalid_cross_morphology -- --nocapture
```

Expected: FAIL because the new parameters and validation do not exist.

- [ ] **Step 3: Add configuration fields**

Add to `CrossDetectParams` and `CrossDetectConfig`:

```rust
pub close_kernel_size: i32,
pub dilate_kernel_size: i32,
pub dilate_iterations: i32,
```

Map them in `CrossDetectConfig::from_params`, add values to test constructors,
and configure:

```toml
close_kernel_size = 5
dilate_kernel_size = 3
dilate_iterations = 1
```

- [ ] **Step 4: Add validation**

Validate both kernel sizes with:

```rust
fn valid_morphology_kernel(size: i32) -> bool {
    (1..=31).contains(&size) && size % 2 == 1
}
```

Reject `dilate_iterations` outside `0..=5`.

- [ ] **Step 5: Run configuration tests**

Run:

```bash
cargo test register_func_rejects_invalid_cross_morphology -- --nocapture
cargo test device::vision::tests::config -- --nocapture
```

Expected: PASS.

### Task 2: Replace Opening With Closing And Dilation

**Files:**
- Modify: `src/device/vision/cross.rs`
- Modify: `src/device/vision/tests/cross.rs`

- [ ] **Step 1: Write failing mask behavior test**

Create a thin broken-ring frame, run `analyze_cross_frame`, and assert:

```rust
assert!(analysis.result.valid);
assert!(core::count_non_zero(&analysis.black_mask)? > raw_foreground);
```

Also assert the fitted center remains within 8 pixels of the drawn center.

- [ ] **Step 2: Run the test and verify RED**

Run:

```bash
cargo test cross_morphology_reconnects_and_thickens_broken_arcs -- --nocapture
```

Expected: FAIL because current opening removes thin foreground or the mask does
not gain the configured thickness.

- [ ] **Step 3: Implement the mask pipeline**

Change `black_mask` to accept `&CrossDetectConfig`. After inverse threshold:

```rust
let close_kernel = imgproc::get_structuring_element(
    imgproc::MORPH_ELLIPSE,
    Size::new(config.close_kernel_size, config.close_kernel_size),
    Point::new(-1, -1),
)?;
imgproc::morphology_ex(
    &thresholded,
    &mut closed,
    imgproc::MORPH_CLOSE,
    &close_kernel,
    Point::new(-1, -1),
    1,
    core::BORDER_CONSTANT,
    Scalar::all(0.0),
)?;
```

If `dilate_iterations > 0`, dilate `closed` with the configured elliptical
kernel. Otherwise return `closed`.

- [ ] **Step 4: Run Cross tests**

Run:

```bash
cargo test device::vision::tests::cross -- --nocapture
```

Expected: all Cross tests PASS.

### Task 3: Verify And Commit

**Files:**
- Modify: `docs/superpowers/plans/2026-06-12-cross-mask-morphology.md`

- [ ] **Step 1: Format and compile**

Run:

```bash
cargo fmt --check
cargo check
```

Expected: both exit zero without new warnings.

- [ ] **Step 2: Run full regression**

Run:

```bash
cargo test --all-targets
```

Expected: all non-hardware tests PASS; camera/GUI tests remain ignored.

- [ ] **Step 3: Check the diff**

Run:

```bash
git diff --check
git status --short
```

Expected: no whitespace errors; the user's `config/device.toml` change remains
unstaged.

- [ ] **Step 4: Commit**

```bash
git add config/functions.toml src/config/type.rs src/device/vision/config.rs \
  src/device/vision/cross.rs src/device/vision/tests/cross.rs \
  src/func/functions.rs src/func/register.rs \
  docs/superpowers/plans/2026-06-12-cross-mask-morphology.md
git commit -m "feat: thicken cross ring mask"
```
