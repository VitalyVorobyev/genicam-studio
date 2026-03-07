import { useEffect, useRef } from "react";
import { fitContain } from "./viewerUtils";

interface ViewerCanvasProps {
  wsUrl: string;
  onFrameStats: (fps: number) => void;
}

export function ViewerCanvas({ wsUrl, onFrameStats }: ViewerCanvasProps) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const wrapRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    const ws = new WebSocket(wsUrl);
    ws.binaryType = "arraybuffer";

    // Rolling 1-second FPS window
    const frameTimes: number[] = [];

    ws.onmessage = (event) => {
      if (typeof event.data === "string") {
        // JSON info frame — ignore for now
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

          const boxW = wrap.clientWidth;
          const boxH = wrap.clientHeight;
          const fit = fitContain(bitmap.width, bitmap.height, boxW, boxH);

          canvas.width = fit.width;
          canvas.height = fit.height;
          canvas.style.left = `${fit.left}px`;
          canvas.style.top = `${fit.top}px`;

          const ctx = canvas.getContext("2d");
          if (ctx) {
            ctx.drawImage(bitmap, 0, 0, fit.width, fit.height);
          }
          bitmap.close();

          // Update rolling FPS
          const now = performance.now();
          frameTimes.push(now);
          while (frameTimes.length > 0 && now - frameTimes[0] > 1000) {
            frameTimes.shift();
          }
          onFrameStats(frameTimes.length);
        })
        .catch(() => {
          // Ignore individual frame decode errors
        });
    };

    return () => {
      ws.close();
    };
  }, [wsUrl, onFrameStats]);

  return (
    <div ref={wrapRef} className="iv-canvas-wrap">
      <canvas ref={canvasRef} />
    </div>
  );
}
