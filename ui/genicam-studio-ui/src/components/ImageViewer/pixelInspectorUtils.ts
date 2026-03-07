/**
 * Pixel sampling utilities for the Image Viewer crosshair inspector.
 *
 * Samples raw bytes from the WebSocket frame buffer at a given image-space
 * coordinate. The buffer holds raw BMP-encoded bytes as received from the
 * genicam-ws-streamer.
 *
 * TODO(ST-02): The streamer currently delivers 8-bit BMP frames for all
 * pixel formats. Mono16 bytes here are actually 8-bit downsampled BMP pixel
 * data. Full 16-bit depth will be available after ST-02 (multi-format encoder).
 */

import type { ImageCoords } from "./viewerUtils";

/** A sampled pixel value from the raw frame buffer. */
export type PixelSample =
  | { kind: "mono"; value: number }
  | { kind: "rgb"; r: number; g: number; b: number };

/**
 * Sample a single pixel from the raw frame buffer at image-space coordinates.
 *
 * Supported formats:
 * - Mono8, BayerRG8, BayerGR8, BayerBG8, BayerGB8: 1 byte per pixel
 * - Mono16: 2 bytes per pixel, little-endian
 *   (TODO(ST-02): bytes are 8-bit BMP data until ST-02 is complete)
 * - RGB8: 3 bytes per pixel, R G B order
 * - BGR8: 3 bytes per pixel, B G R order
 *
 * Returns null for out-of-bounds coordinates, unknown format, or invalid inputs.
 */
export function samplePixel(
  bytes: Uint8Array,
  x: number,
  y: number,
  imgW: number,
  imgH: number,
  format: string,
): PixelSample | null {
  if (x < 0 || y < 0 || x >= imgW || y >= imgH) return null;

  switch (format) {
    case "Mono8":
    case "BayerRG8":
    case "BayerGR8":
    case "BayerBG8":
    case "BayerGB8": {
      const idx = y * imgW + x;
      if (idx >= bytes.length) return null;
      return { kind: "mono", value: bytes[idx] };
    }
    case "Mono16": {
      // TODO(ST-02): streamer delivers 8-bit BMP; 16-bit depth only available after ST-02
      const idx = (y * imgW + x) * 2;
      if (idx + 1 >= bytes.length) return null;
      const value = bytes[idx] | (bytes[idx + 1] << 8);
      return { kind: "mono", value };
    }
    case "RGB8": {
      const idx = (y * imgW + x) * 3;
      if (idx + 2 >= bytes.length) return null;
      return { kind: "rgb", r: bytes[idx], g: bytes[idx + 1], b: bytes[idx + 2] };
    }
    case "BGR8": {
      const idx = (y * imgW + x) * 3;
      if (idx + 2 >= bytes.length) return null;
      return { kind: "rgb", r: bytes[idx + 2], g: bytes[idx + 1], b: bytes[idx] };
    }
    default:
      return null;
  }
}

/**
 * Format a pixel sample as a human-readable string.
 * - Mono: the integer value
 * - RGB: "R:N G:N B:N"
 */
export function formatPixelSample(sample: PixelSample): string {
  if (sample.kind === "mono") {
    return String(sample.value);
  }
  return `R:${sample.r} G:${sample.g} B:${sample.b}`;
}

/** Hover info passed from ViewerCanvas to ImageViewer and then to the status bar. */
export interface PixelHoverInfo {
  coords: ImageCoords;
  sample: PixelSample | null;
  formatted: string;
}
