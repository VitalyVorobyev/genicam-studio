/** Result of fitting an image into a container using contain-scaling. */
export interface FitContainResult {
  width: number;
  height: number;
  left: number;
  top: number;
}

/**
 * Compute letterbox-fit dimensions for an image inside a container.
 * Never upscales: scale is capped at 1.0.
 * Centers the scaled image within the container box.
 * Returns zero-sized result when either container dimension is zero.
 */
export function fitContain(
  imgW: number,
  imgH: number,
  boxW: number,
  boxH: number,
): FitContainResult {
  if (boxW <= 0 || boxH <= 0 || imgW <= 0 || imgH <= 0) {
    return { width: 0, height: 0, left: 0, top: 0 };
  }
  const scale = Math.min(boxW / imgW, boxH / imgH, 1);
  const width = Math.round(imgW * scale);
  const height = Math.round(imgH * scale);
  const left = Math.round((boxW - width) / 2);
  const top = Math.round((boxH - height) / 2);
  return { width, height, left, top };
}

/**
 * Format a frames-per-second value to two decimal places.
 * Example: 30 → "30.00 fps"
 */
export function formatFps(fps: number): string {
  return `${fps.toFixed(2)} fps`;
}

/**
 * Format image dimensions as a human-readable string.
 * Uses the Unicode multiplication sign × (U+00D7), not letter x.
 * Example: 1920, 1080 → "1920 × 1080"
 */
export function formatResolution(w: number, h: number): string {
  return `${w} \u00d7 ${h}`;
}
