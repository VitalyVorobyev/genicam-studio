import React, { useEffect, useRef, useState } from "react";
import { fitContain, mouseToImageCoords, mouseToImageCoordsClamped } from "./viewerUtils";
import { useZoomPan } from "./useZoomPan";
import type { ZoomPanState } from "./useZoomPan";
import { buildTransform } from "./zoomPanUtils";
import { samplePixel, formatPixelSample } from "./pixelInspectorUtils";
import type { PixelHoverInfo } from "./pixelInspectorUtils";
import { CrosshairOverlay } from "./CrosshairOverlay";
import { HistogramOverlay } from "./HistogramOverlay";
import { RoiOverlay } from "./RoiOverlay";
import type { ScreenRect } from "./RoiOverlay";
import { buildImageRect, clampRoiToImage } from "./roiUtils";
import type { ImageRect } from "./roiUtils";
import { LineOverlay } from "./LineOverlay";
import type { ScreenLine } from "./LineOverlay";
import { LineProfilePanel } from "./LineProfilePanel";
import type { LineSegment } from "./lineProfileUtils";

export type { PixelHoverInfo };

export interface StreamInfoFrame {
  type: "info";
  width: number;
  height: number;
  pixel_format: string;
  encoding: string;
}

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
  onStreamInfoChange?: (info: StreamInfoFrame) => void;
  showHistogram?: boolean;
  roiMode?: boolean;
  onRoiSelect?: (roi: ImageRect | null) => void;
  lineMode?: boolean;
  onLineSelect?: (seg: LineSegment | null) => void;
  lineSegment?: LineSegment | null;
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
  onStreamInfoChange,
  showHistogram,
  roiMode,
  onRoiSelect,
  lineMode,
  onLineSelect,
  lineSegment,
}: ViewerCanvasProps) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const wrapRef = useRef<HTMLDivElement | null>(null);
  const lastFrameRef = useRef<Uint8Array | null>(null);

  const [imgSize, setImgSize] = useState({ w: 0, h: 0 });
  const [boxSize, setBoxSize] = useState({ w: 0, h: 0 });
  const [wrapMouse, setWrapMouse] = useState<{ x: number; y: number } | null>(
    null,
  );

  // ── ROI drag state ───────────────────────────────────────────────────────────
  const roiDragRef = useRef({ active: false, startX: 0, startY: 0 });
  const [roiOverlayRect, setRoiOverlayRect] = useState<ScreenRect | null>(null);

  // Clear overlay when ROI mode is turned off
  useEffect(() => {
    if (!roiMode) {
      setRoiOverlayRect(null);
    }
  }, [roiMode]);

  // ── Line drag state ──────────────────────────────────────────────────────────
  const lineDragRef = useRef({ active: false, startX: 0, startY: 0 });
  const [lineOverlay, setLineOverlay] = useState<ScreenLine | null>(null);

  // Clear overlay when line mode is turned off
  useEffect(() => {
    if (!lineMode) {
      setLineOverlay(null);
    }
  }, [lineMode]);

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
        try {
          const parsed = JSON.parse(event.data) as StreamInfoFrame;
          if (parsed.type === "info") {
            onStreamInfoChange?.(parsed);
          }
        } catch {
          // ignore malformed text messages
        }
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
  }, [wsUrl, onFrameStats, onFrame, onStreamInfoChange]);

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

  // ── ROI pointer handlers ─────────────────────────────────────────────────────

  const handleRoiPointerDown = (e: React.PointerEvent<HTMLDivElement>) => {
    if (e.button !== 0) return;
    e.currentTarget.setPointerCapture(e.pointerId);
    const bRect = e.currentTarget.getBoundingClientRect();
    const x = e.clientX - bRect.left;
    const y = e.clientY - bRect.top;
    roiDragRef.current = { active: true, startX: x, startY: y };
    setRoiOverlayRect({ x1: x, y1: y, x2: x, y2: y });
    onRoiSelect?.(null);
  };

  const handleRoiPointerMove = (e: React.PointerEvent<HTMLDivElement>) => {
    if (!roiDragRef.current.active) return;
    const bRect = e.currentTarget.getBoundingClientRect();
    const x = e.clientX - bRect.left;
    const y = e.clientY - bRect.top;
    setRoiOverlayRect({
      x1: roiDragRef.current.startX,
      y1: roiDragRef.current.startY,
      x2: x,
      y2: y,
    });
  };

  const handleRoiPointerUp = (e: React.PointerEvent<HTMLDivElement>) => {
    if (!roiDragRef.current.active) return;
    roiDragRef.current.active = false;

    const bRect = e.currentTarget.getBoundingClientRect();
    const endX = e.clientX - bRect.left;
    const endY = e.clientY - bRect.top;

    setRoiOverlayRect({
      x1: roiDragRef.current.startX,
      y1: roiDragRef.current.startY,
      x2: endX,
      y2: endY,
    });

    const { scale, panX, panY, fitScale } = zoomPan.state;
    const params = {
      boxW: boxSize.w,
      boxH: boxSize.h,
      imgW: imgSize.w,
      imgH: imgSize.h,
      scale,
      fitScale,
      panX,
      panY,
    };

    const startPt = mouseToImageCoordsClamped({
      ...params,
      mouseX: roiDragRef.current.startX,
      mouseY: roiDragRef.current.startY,
    });
    const endPt = mouseToImageCoordsClamped({ ...params, mouseX: endX, mouseY: endY });

    if (startPt && endPt) {
      const raw = buildImageRect(startPt, endPt);
      const clamped = clampRoiToImage(raw, imgSize.w, imgSize.h);
      onRoiSelect?.(clamped);
    }
  };

  // ── Line pointer handlers ────────────────────────────────────────────────────

  const handleLinePointerDown = (e: React.PointerEvent<HTMLDivElement>) => {
    if (e.button !== 0) return;
    e.currentTarget.setPointerCapture(e.pointerId);
    const bRect = e.currentTarget.getBoundingClientRect();
    const x = e.clientX - bRect.left;
    const y = e.clientY - bRect.top;
    lineDragRef.current = { active: true, startX: x, startY: y };
    setLineOverlay({ x1: x, y1: y, x2: x, y2: y });
    onLineSelect?.(null);
  };

  const handleLinePointerMove = (e: React.PointerEvent<HTMLDivElement>) => {
    if (!lineDragRef.current.active) return;
    const bRect = e.currentTarget.getBoundingClientRect();
    const x = e.clientX - bRect.left;
    const y = e.clientY - bRect.top;
    setLineOverlay({
      x1: lineDragRef.current.startX,
      y1: lineDragRef.current.startY,
      x2: x,
      y2: y,
    });
  };

  const handleLinePointerUp = (e: React.PointerEvent<HTMLDivElement>) => {
    if (!lineDragRef.current.active) return;
    lineDragRef.current.active = false;

    const bRect = e.currentTarget.getBoundingClientRect();
    const endX = e.clientX - bRect.left;
    const endY = e.clientY - bRect.top;

    // Keep overlay line visible after release
    setLineOverlay({
      x1: lineDragRef.current.startX,
      y1: lineDragRef.current.startY,
      x2: endX,
      y2: endY,
    });

    const { scale, panX, panY, fitScale } = zoomPan.state;
    const params = {
      boxW: boxSize.w,
      boxH: boxSize.h,
      imgW: imgSize.w,
      imgH: imgSize.h,
      scale,
      fitScale,
      panX,
      panY,
    };

    const startPt = mouseToImageCoordsClamped({
      ...params,
      mouseX: lineDragRef.current.startX,
      mouseY: lineDragRef.current.startY,
    });
    const endPt = mouseToImageCoordsClamped({ ...params, mouseX: endX, mouseY: endY });

    if (startPt && endPt) {
      onLineSelect?.({ x0: startPt.x, y0: startPt.y, x1: endPt.x, y1: endPt.y });
    }
  };

  // ── Cursor and class derivation ─────────────────────────────────────────────

  const isZoomed = imgSize.w > 0 && zoomPan.state.scale > zoomPan.state.fitScale;
  const isDragging = zoomPan.isDraggingRef.current;
  let cursorClass = "";
  if (lineMode || roiMode) {
    cursorClass = "iv-canvas-wrap--crosshair";
  } else if (isZoomed) {
    cursorClass = isDragging ? "iv-canvas-wrap--grabbing" : "iv-canvas-wrap--grab";
  }
  const streamClass = isStreaming ? "iv-canvas-wrap--active" : "iv-canvas-wrap--idle";

  // Choose pointer handlers: lineMode > roiMode > zoom/pan
  const pointerHandlers = lineMode
    ? {
        onPointerDown: handleLinePointerDown,
        onPointerMove: handleLinePointerMove,
        onPointerUp: handleLinePointerUp,
        onDoubleClick: undefined,
      }
    : roiMode
    ? {
        onPointerDown: handleRoiPointerDown,
        onPointerMove: handleRoiPointerMove,
        onPointerUp: handleRoiPointerUp,
        onDoubleClick: undefined,
      }
    : {
        onPointerDown: zoomPan.onPointerDown,
        onPointerMove: zoomPan.onPointerMove,
        onPointerUp: zoomPan.onPointerUp,
        onDoubleClick: zoomPan.onDoubleClick,
      };

  const suppressCrosshair = lineMode || roiMode;

  return (
    <div
      ref={wrapRef}
      className={`iv-canvas-wrap ${streamClass}${cursorClass ? ` ${cursorClass}` : ""}`}
      {...pointerHandlers}
      onMouseMove={handleMouseMove}
      onMouseLeave={handleMouseLeave}
    >
      <canvas ref={canvasRef} />
      <CrosshairOverlay
        mouseX={suppressCrosshair ? null : (wrapMouse?.x ?? null)}
        mouseY={suppressCrosshair ? null : (wrapMouse?.y ?? null)}
      />
      <RoiOverlay rect={roiMode ? roiOverlayRect : null} />
      <LineOverlay line={lineMode ? lineOverlay : null} />
      {snapshotRef && (
        <HistogramOverlay
          frameRef={snapshotRef}
          visible={showHistogram ?? false}
        />
      )}
      {snapshotRef && (
        <LineProfilePanel
          frameRef={snapshotRef}
          lineSegment={lineSegment ?? null}
          visible={lineMode ?? false}
        />
      )}
    </div>
  );
}
