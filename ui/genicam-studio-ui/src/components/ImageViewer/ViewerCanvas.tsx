import React, { useEffect, useRef, useState } from "react";
import { fitContain, mouseToImageCoords } from "./viewerUtils";
import { useZoomPan } from "./useZoomPan";
import type { ZoomPanState } from "./useZoomPan";
import { buildTransform } from "./zoomPanUtils";
import { samplePixel, formatPixelSample } from "./pixelInspectorUtils";
import type { PixelHoverInfo } from "./pixelInspectorUtils";
import { CrosshairOverlay } from "./CrosshairOverlay";

export type { PixelHoverInfo };

interface ViewerCanvasProps {
  wsUrl: string;
  onFrameStats: (fps: number) => void;
  onFrame?: () => void;
  onZoomPanChange?: (state: ZoomPanState) => void;
  pixelFormat?: string;
  onPixelHover?: (info: PixelHoverInfo | null) => void;
  resetZoomRef?: React.RefObject<(() => void) | null>;
  isStreaming?: boolean;
  snapshotRef?: React.RefObject<Uint8Array | null>;
}

export function ViewerCanvas({
  wsUrl,
  onFrameStats,
  onFrame,
  onZoomPanChange,
  pixelFormat,
  onPixelHover,
  resetZoomRef,
  isStreaming,
  snapshotRef,
}: ViewerCanvasProps) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const wrapRef = useRef<HTMLDivElement | null>(null);
  const lastFrameRef = useRef<Uint8Array | null>(null);

  const [imgSize, setImgSize] = useState({ w: 0, h: 0 });
  const [boxSize, setBoxSize] = useState({ w: 0, h: 0 });
  const [wrapMouse, setWrapMouse] = useState<{ x: number; y: number } | null>(
    null,
  );

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

  // Expose reset-zoom callback to parent via ref
  useEffect(() => {
    if (!resetZoomRef) return;
    resetZoomRef.current = zoomPan.onDoubleClick;
    return () => {
      resetZoomRef.current = null;
    };
  }, [resetZoomRef, zoomPan.onDoubleClick]);

  // WebSocket + render loop
  useEffect(() => {
    const ws = new WebSocket(wsUrl);
    ws.binaryType = "arraybuffer";

    const frameTimes: number[] = [];

    ws.onmessage = (event) => {
      if (typeof event.data === "string") {
        return;
      }

      // Store raw bytes for pixel sampling before decoding
      lastFrameRef.current = new Uint8Array(event.data);
      if (snapshotRef) snapshotRef.current = lastFrameRef.current;

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

  const handleMouseMove = (e: React.MouseEvent<HTMLDivElement>) => {
    const rect = e.currentTarget.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    setWrapMouse({ x, y });

    if (!onPixelHover) return;

    const { scale, panX, panY, fitScale } = zoomPan.state;
    const coords = mouseToImageCoords({
      mouseX: x,
      mouseY: y,
      boxW: boxSize.w,
      boxH: boxSize.h,
      imgW: imgSize.w,
      imgH: imgSize.h,
      scale,
      fitScale,
      panX,
      panY,
    });

    if (!coords) {
      onPixelHover(null);
      return;
    }

    const frame = lastFrameRef.current;
    const fmt = pixelFormat ?? "Mono8";
    const sample =
      frame
        ? samplePixel(frame, coords.x, coords.y, imgSize.w, imgSize.h, fmt)
        : null;
    const formatted = sample ? formatPixelSample(sample) : "";
    onPixelHover({ coords, sample, formatted });
  };

  const handleMouseLeave = () => {
    setWrapMouse(null);
    onPixelHover?.(null);
  };

  // Cursor class based on zoom level and drag state
  const isZoomed = imgSize.w > 0 && zoomPan.state.scale > zoomPan.state.fitScale;
  const isDragging = zoomPan.isDraggingRef.current;
  const cursorClass = isZoomed
    ? isDragging
      ? "iv-canvas-wrap--grabbing"
      : "iv-canvas-wrap--grab"
    : "";
  const streamClass = isStreaming ? "iv-canvas-wrap--active" : "iv-canvas-wrap--idle";

  return (
    <div
      ref={wrapRef}
      className={`iv-canvas-wrap ${streamClass}${cursorClass ? ` ${cursorClass}` : ""}`}
      onPointerDown={zoomPan.onPointerDown}
      onPointerMove={zoomPan.onPointerMove}
      onPointerUp={zoomPan.onPointerUp}
      onDoubleClick={zoomPan.onDoubleClick}
      onMouseMove={handleMouseMove}
      onMouseLeave={handleMouseLeave}
    >
      <canvas ref={canvasRef} />
      <CrosshairOverlay
        mouseX={wrapMouse?.x ?? null}
        mouseY={wrapMouse?.y ?? null}
      />
    </div>
  );
}
