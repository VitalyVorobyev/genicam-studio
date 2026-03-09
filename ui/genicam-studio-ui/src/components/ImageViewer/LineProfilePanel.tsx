import { useEffect, useRef } from "react";
import { sampleLineProfile, renderLineProfile } from "./lineProfileUtils";
import type { LineSegment } from "./lineProfileUtils";

const CANVAS_W = 260;
const CANVAS_H = 80;
const REFRESH_MS = 200; // ~5 fps refresh

interface LineProfilePanelProps {
  frameRef: React.RefObject<Uint8Array | null>;
  lineSegment: LineSegment | null;
  visible: boolean;
}

/**
 * Floating line intensity profile panel for the Image Viewer.
 *
 * Positioned at the bottom-left of the canvas area (histogram occupies
 * bottom-right). Polls the latest BMP frame via `frameRef` every 200 ms
 * and renders the intensity profile along `lineSegment`.
 *
 * - 8bpp BMP frames → single gray polyline.
 * - 24bpp BMP frames → per-channel R, G, B polylines.
 * pointer-events: none — does not block canvas interactions.
 */
export function LineProfilePanel({ frameRef, lineSegment, visible }: LineProfilePanelProps) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);

  useEffect(() => {
    if (!visible || !lineSegment) return;

    const tick = () => {
      const bytes = frameRef.current;
      if (!bytes) return;
      const data = sampleLineProfile(
        bytes,
        lineSegment.x0,
        lineSegment.y0,
        lineSegment.x1,
        lineSegment.y1,
      );
      if (!data) return;
      const canvas = canvasRef.current;
      if (!canvas) return;
      const ctx = canvas.getContext("2d");
      if (!ctx) return;
      renderLineProfile(ctx, data, CANVAS_W, CANVAS_H);
    };

    tick(); // render immediately when shown or segment changes
    const id = setInterval(tick, REFRESH_MS);
    return () => clearInterval(id);
  }, [visible, lineSegment, frameRef]);

  if (!visible || !lineSegment) return null;

  return (
    <div className="iv-line-profile" aria-label="Line profile">
      <canvas
        ref={canvasRef}
        width={CANVAS_W}
        height={CANVAS_H}
        className="iv-line-profile__canvas"
      />
    </div>
  );
}
