import { useEffect, useRef, useState } from "react";
import { fitContain } from "./viewerUtils";
import { useZoomPan } from "./useZoomPan";
import type { ZoomPanState } from "./useZoomPan";
import { buildTransform } from "./zoomPanUtils";

interface ViewerCanvasProps {
  wsUrl: string;
  onFrameStats: (fps: number) => void;
  onFrame?: () => void;
  onZoomPanChange?: (state: ZoomPanState) => void;
}

export function ViewerCanvas({
  wsUrl,
  onFrameStats,
  onFrame,
  onZoomPanChange,
}: ViewerCanvasProps) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const wrapRef = useRef<HTMLDivElement | null>(null);

  const [imgSize, setImgSize] = useState({ w: 0, h: 0 });
  const [boxSize, setBoxSize] = useState({ w: 0, h: 0 });

  // Track box size via ResizeObserver
  useEffect(() => {
    const wrap = wrapRef.current;
    if (!wrap) return;

    const observer = new ResizeObserver((entries) => {
      const entry = entries[0];
      if (!entry) return;
      const { width, height } = entry.contentRect;
      setBoxSize({ w: Math.round(width), h: Math.round(height) });
    });
    observer.observe(wrap);

    // Seed initial size immediately
    setBoxSize({ w: Math.round(wrap.clientWidth), h: Math.round(wrap.clientHeight) });

    return () => observer.disconnect();
  }, []);

  const zoomPan = useZoomPan(imgSize.w, imgSize.h, boxSize.w, boxSize.h);

  // Refs for stale-closure-safe wheel handler registration
  const zoomPanRef = useRef(zoomPan);
  zoomPanRef.current = zoomPan;

  // Attach native wheel listener (passive: false so we can preventDefault)
  useEffect(() => {
    const wrap = wrapRef.current;
    if (!wrap) return;
    const handler = zoomPan.getWheelHandler(wrap);
    wrap.addEventListener("wheel", handler, { passive: false });
    return () => wrap.removeEventListener("wheel", handler);
    // Re-attach when imgSize/boxSize change so the handler captures fresh dims
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [imgSize.w, imgSize.h, boxSize.w, boxSize.h]);

  // Apply CSS transform whenever zoom/pan state changes
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const { scale, panX, panY, fitScale } = zoomPan.state;
    canvas.style.transform = buildTransform(panX, panY, scale, fitScale);
  }, [zoomPan.state]);

  // Emit zoom/pan state to parent
  useEffect(() => {
    onZoomPanChange?.(zoomPan.state);
  }, [zoomPan.state, onZoomPanChange]);

  // WebSocket + render loop
  useEffect(() => {
    const ws = new WebSocket(wsUrl);
    ws.binaryType = "arraybuffer";

    const frameTimes: number[] = [];

    ws.onmessage = (event) => {
      if (typeof event.data === "string") {
        return;
      }

      const blob = new Blob([event.data]);
      createImageBitmap(blob)
        .then((bitmap) => {
          const canvas = canvasRef.current;
          const wrap = wrapRef.current;
          if (!canvas || !wrap) {
            bitmap.close();
            return;
          }

          const newImgW = bitmap.width;
          const newImgH = bitmap.height;

          // Update canvas intrinsic size to fitContain dimensions
          const boxW = wrap.clientWidth;
          const boxH = wrap.clientHeight;
          const fit = fitContain(newImgW, newImgH, boxW, boxH);

          canvas.width = fit.width;
          canvas.height = fit.height;

          // Centre the canvas using CSS; transform is managed by zoom/pan effect
          canvas.style.left = "50%";
          canvas.style.top = "50%";
          canvas.style.transformOrigin = "center center";

          const ctx = canvas.getContext("2d");
          if (ctx) {
            ctx.drawImage(bitmap, 0, 0, fit.width, fit.height);
          }
          bitmap.close();

          // Update imgSize state only when dimensions actually change
          setImgSize((prev) => {
            if (prev.w !== newImgW || prev.h !== newImgH) {
              return { w: newImgW, h: newImgH };
            }
            return prev;
          });

          // Rolling 1-second FPS window
          const now = performance.now();
          frameTimes.push(now);
          while (frameTimes.length > 0 && now - frameTimes[0] > 1000) {
            frameTimes.shift();
          }
          onFrameStats(frameTimes.length);
          onFrame?.();
        })
        .catch(() => {
          // Ignore individual frame decode errors
        });
    };

    return () => {
      ws.close();
    };
  }, [wsUrl, onFrameStats, onFrame]);

  // Cursor class based on zoom level and drag state
  const isZoomed = imgSize.w > 0 && zoomPan.state.scale > zoomPan.state.fitScale;
  const isDragging = zoomPan.isDraggingRef.current;
  const cursorClass = isZoomed
    ? isDragging
      ? "iv-canvas-wrap--grabbing"
      : "iv-canvas-wrap--grab"
    : "";

  return (
    <div
      ref={wrapRef}
      className={`iv-canvas-wrap${cursorClass ? ` ${cursorClass}` : ""}`}
      onPointerDown={zoomPan.onPointerDown}
      onPointerMove={zoomPan.onPointerMove}
      onPointerUp={zoomPan.onPointerUp}
      onDoubleClick={zoomPan.onDoubleClick}
    >
      <canvas ref={canvasRef} />
    </div>
  );
}
