# Vision Navigation Targets Design

**Date:** 2026-06-08

**Status:** Design decisions confirmed in conversation; implementation pending.

**Scope:** Implement independent vision functions for cross marker detection, dual black-line lane detection, and inner circle target detection. Add a shared ground-coordinate transform utility.

**Deferred:** Colored cylinder / `color_cylinder_detect` is recorded as a later feature and is not part of this implementation.

## Goals

The engine should provide standalone function calls for the electric-control board. The engine does not own driving state, target switching, or PID control. The board decides when to call each function and how to use the returned measurement.

This work adds three function results:

```text
CROSS,valid,unit,lateral,forward,yaw_cdeg,score
LANE,valid,unit,lateral,heading_cdeg,width,score
INNER_CIRCLE,valid,unit,lateral,forward,score
```

The `unit` field is required:

- `PX`: output values are pixel-domain offsets.
- `MM`: output values are ground-plane millimeter coordinates after perspective mapping.
- `NA`: invalid result.

Invalid results must not silently fall back from `MM` to `PX`.

## Coordinate Transform

The camera is fixed but tilted, so image coordinates are not linearly related to real ground distance. A shared utility will convert image points to ground coordinates using a homography-like perspective transform.

Each function can opt into conversion:

```toml
use_ground_transform = true
```

When false, functions return pixel offsets. When true, functions call the shared transform utility and return millimeters. The transform is configured with four or more point pairs:

```toml
[functions.entries.params.ground_transform]
image_points = [[320.0, 180.0], [1600.0, 180.0], [1800.0, 900.0], [120.0, 900.0]]
ground_points_mm = [[-600.0, 2000.0], [600.0, 2000.0], [600.0, 200.0], [-600.0, 200.0]]
```

Ground coordinates use:

- `lateral`: right is positive, left is negative.
- `forward`: forward from the vehicle/camera reference is positive.

Pixel mode should preserve the same sign convention where possible:

- `lateral_px = point_x - target_x`.
- `forward_px = target_y - point_y`, so objects higher in the image are treated as farther forward.

Each function has a `target_correction` offset for the image reference point:

```toml
target_correction = { x = 0, y = 0 }
```

The reference point is:

```text
target_x = frame_width / 2 + target_correction.x
target_y = frame_height / 2 + target_correction.y
```

## Cross Detection

The cross marker is a black plus sign on the ground. The real camera may see it from a tilted, distant perspective, so the detector should not depend on a perfectly axis-aligned plus shape.

Detection strategy:

1. Convert frame to grayscale.
2. Threshold black tape.
3. Use contours and/or fitted line segments to find a plus-like component.
4. Score candidates by black area, aspect, two-arm orthogonality, and center consistency.
5. Compute a center point.
6. Compute an orientation angle.
7. Convert center to either pixel offsets or ground millimeters.

Output:

```text
CROSS,valid,unit,lateral,forward,yaw_cdeg,score
```

Examples:

```text
CROSS,1,MM,-120,850,1530,91
CROSS,1,PX,-42,180,1530,91
CROSS,0,NA,0,0,0,0
```

`yaw_cdeg` is centidegrees. For a symmetric plus sign, orientation is ambiguous modulo 90 degrees; the implementation should choose the arm closest to the configured forward direction and report a lower score when orientation is unclear. The electric-control board may use only `lateral` and `forward` if it does not need yaw.

## Lane Detection

The lane task detects two black line boundaries and returns a navigation measurement. It does not decide when to switch to cylinder or ring detection; all switching is owned by the electric-control board.

Detection strategy:

1. Threshold black tape.
2. Focus on a configurable lower/middle region of interest.
3. Extract left and right boundary points using scanning rows or contours.
4. Fit left and right boundary lines/curves.
5. Derive the lane centerline at a configurable lookahead position.
6. Convert the centerline measurement to pixel offsets or ground millimeters.

Output:

```text
LANE,valid,unit,lateral,heading_cdeg,width,score
```

Examples:

```text
LANE,1,MM,-80,620,920,88
LANE,1,PX,23,-540,310,82
LANE,0,NA,0,0,0,0
```

Fields:

- `lateral`: lane center relative to the reference point.
- `heading_cdeg`: lane centerline heading error in centidegrees.
- `width`: lane width at the measurement row or lookahead point, in the same unit as `unit`.
- `score`: confidence from boundary quality, width consistency, and fit residual.

## Inner Circle Detection

The second image contains nested black circular rings and a central letter. The detector should ignore the letter and return the center of the innermost reliable circle/ring.

Detection strategy:

1. Convert frame to grayscale.
2. Threshold black lines.
3. Use contour hierarchy (`RETR_TREE`) and ellipse fitting.
4. Filter by circularity/ellipse aspect and nesting.
5. Ignore small central letter contours by requiring ring-like contour size and circularity.
6. Select the innermost valid ring and return its center.

Output:

```text
INNER_CIRCLE,valid,unit,lateral,forward,score
```

Examples:

```text
INNER_CIRCLE,1,MM,-35,720,92
INNER_CIRCLE,1,PX,-18,146,92
INNER_CIRCLE,0,NA,0,0,0
```

## Manual Tests

Each visual feature needs an ignored GUI test that reads only config and shows every important CV step with `imshow`.

Required manual tests:

- `show_cross_detect_cv_steps_from_config`
- `show_lane_detect_cv_steps_from_config`
- `show_inner_circle_detect_cv_steps_from_config`
- `tune_ground_transform_from_config`

The tests should show original frame, grayscale frame, black mask, filtered candidates, and final annotated result. They are ignored by default because they require a camera and GUI.

## Web And UART

Function outputs should continue to use `TaskOutput.value_with_image` where a frame is available:

- `value`: the UART string.
- `image`: annotated JPEG data URL for Web debug.
- `text`: human-readable summary.

The message router already sends `TaskOutput.value` to UART and the full payload to Web, so no router change is required unless new output routing behavior is needed.

## Deferred Colored Cylinder Detection

The user clarified that the previous “color ring” target is actually the colored cylinder itself. That should become a later feature, likely named `color_cylinder_detect`.

Possible future output:

```text
COLOR_CYLINDER,valid,color,unit,lateral,forward,score
```

For tilted camera views, the likely target point should be the cylinder bottom center / ground contact point rather than the visual center of the cylinder body. This is deferred and must not block the current three target functions.
