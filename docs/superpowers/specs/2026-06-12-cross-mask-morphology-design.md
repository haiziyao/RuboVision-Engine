# Cross Black-Mask Morphology Design

## Goal

Improve `cross` ring-center stability when printed black arcs are thin, noisy, or
slightly broken. The preprocessing should reconnect short gaps and make retained
black strokes thicker before contour extraction.

## Pipeline

The inverse-threshold mask represents black image regions as white foreground.
Replace the current fixed `3x3 close -> 3x3 open` sequence with:

1. Gaussian blur and inverse threshold, unchanged.
2. Elliptical close to bridge short gaps.
3. Elliptical dilation to thicken the foreground strokes.

Opening is removed because its erosion phase can delete thin arc fragments.

## Configuration

Add these typed `cross` parameters:

- `close_kernel_size`: odd integer in `1..=31`, default `5`.
- `dilate_kernel_size`: odd integer in `1..=31`, default `3`.
- `dilate_iterations`: integer in `0..=5`, default `1`.

`dilate_iterations = 0` disables thickening. Existing threshold, contour fitting,
and scoring behavior remains unchanged.

## Debugging

The existing `cross/black_mask` window continues to show the final mask after
closing and dilation, so tuning changes are visible immediately.

## Validation

- Add configuration validation for odd kernel sizes and bounded iterations.
- Add a synthetic thin, broken-ring test that verifies the processed mask gains
  foreground and still yields the expected center.
- Keep complete-ring, occlusion, color-cylinder, output, and full-project tests
  passing.
