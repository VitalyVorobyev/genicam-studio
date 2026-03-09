import { useEffect, useRef } from "react";
import { computeHistogram, renderHistogram } from "./histogramUtils";

const CANVAS_W = 204;
const CANVAS_H = 84;
const REFRESH_MS = 200; // ~5 fps refresh for histogram

interface HistogramOverlayProps {
  frameRef: React.RefObject<Uint8Array | null>;
  visible: boolean;
}

/**
 * Live histogram overlay for the Image Viewer canvas.
 *
 * Polls the latest BMP frame bytes from `frameRef` every 200 ms and renders
 * the histogram onto a small canvas positioned in the bottom-right corner of
 * the image canvas area. pointer-events: none — does not interfere with
 * zoom/pan or the crosshair overlay.
 *
 * - 8bpp BMP frames → single grayscale histogram.
 * - 24bpp BMP frames → per-channel R, G, B histogram with semi-transparent fills.
 */
export function HistogramOverlay({ frameRef, visible }: HistogramOverlayProps) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);

  useEffect(() => {
    if (!visible) return;

    const tick = () => {
      const bytes = frameRef.current;
      if (!bytes) return;
      const data = computeHistogram(bytes);
      if (!data) return;
      const canvas = canvasRef.current;
      if (!canvas) return;
      const ctx = canvas.getContext("2d");
      if (!ctx) return;
      renderHistogram(ctx, data, CANVAS_W, CANVAS_H);
    };

    tick(); // render immediately when shown
    const id = setInterval(tick, REFRESH_MS);
    return () => clearInterval(id);
  }, [visible, frameRef]);

  if (!visible) return null;

  return (
    <div className="iv-histogram" aria-label="Histogram">
      <canvas
        ref={canvasRef}
        width={CANVAS_W}
        height={CANVAS_H}
        className="iv-histogram__canvas"
      />
    </div>
  );
}
