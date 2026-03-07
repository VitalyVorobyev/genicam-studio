import { describe, it, expect } from "vitest";
import { fitContain, formatFps, formatResolution, formatFrameCount } from "./viewerUtils";

describe("fitContain", () => {
  it("test_fitContain_landscape_smaller_than_container", () => {
    // Image 640×480 inside 1280×960 box — scale capped at 1.0, no upscale
    const result = fitContain(640, 480, 1280, 960);
    expect(result.width).toBe(640);
    expect(result.height).toBe(480);
    expect(result.left).toBe(320);
    expect(result.top).toBe(240);
  });

  it("test_fitContain_landscape_larger_container_constrained_by_width", () => {
    // Image 1920×1080, box 960×1080 — width is the constraint
    const result = fitContain(1920, 1080, 960, 1080);
    expect(result.width).toBe(960);
    expect(result.height).toBe(540);
    expect(result.left).toBe(0);
    expect(result.top).toBe(270);
  });

  it("test_fitContain_landscape_larger_container_constrained_by_height", () => {
    // Image 1920×1080, box 1920×540 — height is the constraint
    const result = fitContain(1920, 1080, 1920, 540);
    expect(result.width).toBe(960);
    expect(result.height).toBe(540);
    expect(result.left).toBe(480);
    expect(result.top).toBe(0);
  });

  it("test_fitContain_square_image", () => {
    // Image 512×512, box 512×512 — scale exactly 1.0
    const result = fitContain(512, 512, 512, 512);
    expect(result.width).toBe(512);
    expect(result.height).toBe(512);
    expect(result.left).toBe(0);
    expect(result.top).toBe(0);
  });

  it("test_fitContain_zero_container", () => {
    // Container is 0×0 — must not divide by zero, returns zeroed result
    const result = fitContain(640, 480, 0, 0);
    expect(result.width).toBe(0);
    expect(result.height).toBe(0);
    expect(result.left).toBe(0);
    expect(result.top).toBe(0);
  });
});

describe("formatFps", () => {
  it("test_formatFps_typical", () => {
    expect(formatFps(29.97)).toBe("29.97 fps");
  });

  it("test_formatFps_integer", () => {
    expect(formatFps(30)).toBe("30.00 fps");
  });

  it("test_formatFps_zero", () => {
    expect(formatFps(0)).toBe("0.00 fps");
  });
});

describe("formatResolution", () => {
  it("test_formatResolution_standard", () => {
    expect(formatResolution(1920, 1080)).toBe("1920 \u00d7 1080");
  });

  it("test_formatResolution_square", () => {
    expect(formatResolution(512, 512)).toBe("512 \u00d7 512");
  });
});

describe("formatFrameCount", () => {
  it("test_formatFrameCount_zero", () => {
    expect(formatFrameCount(0)).toBe("0 frames");
  });

  it("test_formatFrameCount_one", () => {
    expect(formatFrameCount(1)).toBe("1 frame");
  });

  it("test_formatFrameCount_many", () => {
    expect(formatFrameCount(42)).toBe("42 frames");
  });

  it("test_formatFrameCount_large", () => {
    expect(formatFrameCount(1234)).toBe("1,234 frames");
  });
});
