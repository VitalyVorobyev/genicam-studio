import { describe, it, expect } from "vitest";
import { samplePixel, formatPixelSample } from "./pixelInspectorUtils";

// Helper: create a flat Uint8Array of given length, filled with zeros,
// then write specific values at specified byte offsets.
function makeBytes(length: number, patches: Record<number, number> = {}): Uint8Array {
  const arr = new Uint8Array(length);
  for (const [idx, val] of Object.entries(patches)) {
    arr[Number(idx)] = val;
  }
  return arr;
}

describe("samplePixel", () => {
  it("test_samplePixel_mono8_center", () => {
    // 3×3 Mono8 image; pixel at (1,1) is at byte index 1*3+1=4
    const bytes = makeBytes(9, { 4: 128 });
    const result = samplePixel(bytes, 1, 1, 3, 3, "Mono8");
    expect(result).toEqual({ kind: "mono", value: 128 });
  });

  it("test_samplePixel_mono16_little_endian", () => {
    // 2×1 Mono16 image; pixel at (1,0) occupies bytes [2,3]
    // value = 0x01 | (0x02 << 8) = 513
    const bytes = makeBytes(4, { 2: 0x01, 3: 0x02 });
    const result = samplePixel(bytes, 1, 0, 2, 1, "Mono16");
    expect(result).toEqual({ kind: "mono", value: 513 });
  });

  it("test_samplePixel_rgb8", () => {
    // 2×1 RGB8 image; pixel at (1,0) occupies bytes [3,4,5]
    const bytes = makeBytes(6, { 3: 10, 4: 20, 5: 30 });
    const result = samplePixel(bytes, 1, 0, 2, 1, "RGB8");
    expect(result).toEqual({ kind: "rgb", r: 10, g: 20, b: 30 });
  });

  it("test_samplePixel_bayerrg8", () => {
    // BayerRG8 is raw Bayer — no debayer, treated as mono
    // 2×2 image; pixel at (0,1) is at byte index 1*2+0=2
    const bytes = makeBytes(4, { 2: 200 });
    const result = samplePixel(bytes, 0, 1, 2, 2, "BayerRG8");
    expect(result).toEqual({ kind: "mono", value: 200 });
  });

  it("test_samplePixel_out_of_bounds", () => {
    // x < 0 → null
    const bytes = makeBytes(9);
    const result = samplePixel(bytes, -1, 0, 3, 3, "Mono8");
    expect(result).toBeNull();
  });

  it("test_samplePixel_out_of_bounds_right", () => {
    // x >= imgW → null
    const bytes = makeBytes(9);
    const result = samplePixel(bytes, 3, 0, 3, 3, "Mono8");
    expect(result).toBeNull();
  });

  it("test_samplePixel_unknown_format", () => {
    // Unrecognized format → null
    const bytes = makeBytes(4);
    const result = samplePixel(bytes, 0, 0, 2, 2, "YUV422");
    expect(result).toBeNull();
  });

  it("test_samplePixel_bgr8", () => {
    // 1×1 BGR8; bytes at [0,1,2] are B=5, G=10, R=15; r/g/b should be swapped
    const bytes = makeBytes(3, { 0: 5, 1: 10, 2: 15 });
    const result = samplePixel(bytes, 0, 0, 1, 1, "BGR8");
    expect(result).toEqual({ kind: "rgb", r: 15, g: 10, b: 5 });
  });
});

describe("formatPixelSample", () => {
  it("test_formatPixelSample_mono", () => {
    expect(formatPixelSample({ kind: "mono", value: 42 })).toBe("42");
  });

  it("test_formatPixelSample_mono16_max", () => {
    expect(formatPixelSample({ kind: "mono", value: 65535 })).toBe("65535");
  });

  it("test_formatPixelSample_rgb", () => {
    expect(
      formatPixelSample({ kind: "rgb", r: 255, g: 128, b: 0 }),
    ).toBe("R:255 G:128 B:0");
  });

  it("test_formatPixelSample_bayer", () => {
    // Bayer formats produce mono samples — same formatting as Mono8
    expect(formatPixelSample({ kind: "mono", value: 200 })).toBe("200");
  });
});
