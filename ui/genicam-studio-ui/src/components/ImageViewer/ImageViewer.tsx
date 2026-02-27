import { useEffect, useRef, useState } from "react";
import type { StreamerInfo } from "../../device/types";

interface ImageViewerProps {
  streamerInfo: StreamerInfo | null;
}

export function ImageViewer({ streamerInfo }: ImageViewerProps) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const [fpsDisplay, setFpsDisplay] = useState<string>("");
  const [status, setStatus] = useState<"idle" | "connecting" | "streaming" | "error">("idle");
  const [errorMsg, setErrorMsg] = useState<string>("");

  useEffect(() => {
    if (!streamerInfo) {
      setStatus("idle");
      return;
    }

    setStatus("connecting");
    setErrorMsg("");

    const ws = new WebSocket(streamerInfo.ws_url);
    ws.binaryType = "arraybuffer";

    // Rolling FPS counter
    const frameTimes: number[] = [];
    let animationId = 0;

    ws.onopen = () => {
      setStatus("connecting"); // wait for first frame
    };

    ws.onerror = () => {
      setStatus("error");
      setErrorMsg("WebSocket connection failed");
    };

    ws.onclose = () => {
      setStatus("idle");
      cancelAnimationFrame(animationId);
    };

    ws.onmessage = (event) => {
      if (typeof event.data === "string") {
        // Info frame (JSON) — ignore for now
        setStatus("streaming");
        return;
      }

      const blob = new Blob([event.data]);
      createImageBitmap(blob)
        .then((bitmap) => {
          const canvas = canvasRef.current;
          if (!canvas) return;

          // Resize canvas to match bitmap preserving aspect ratio
          const containerW = canvas.parentElement?.clientWidth ?? bitmap.width;
          const containerH = canvas.parentElement?.clientHeight ?? bitmap.height;
          const scale = Math.min(containerW / bitmap.width, containerH / bitmap.height, 1);
          canvas.width = Math.round(bitmap.width * scale);
          canvas.height = Math.round(bitmap.height * scale);

          const ctx = canvas.getContext("2d");
          if (ctx) {
            ctx.drawImage(bitmap, 0, 0, canvas.width, canvas.height);
            bitmap.close();
          }

          // Rolling FPS
          const now = performance.now();
          frameTimes.push(now);
          while (frameTimes.length > 0 && now - frameTimes[0] > 1000) {
            frameTimes.shift();
          }
          setFpsDisplay(`${frameTimes.length} fps`);
          setStatus("streaming");
        })
        .catch(() => {
          // Ignore decode errors for individual frames
        });
    };

    return () => {
      ws.close();
      cancelAnimationFrame(animationId);
    };
  }, [streamerInfo]);

  if (!streamerInfo) {
    return (
      <div className="image-viewer image-viewer--idle">
        <p>Start acquisition to view the live image stream.</p>
      </div>
    );
  }

  return (
    <div className="image-viewer">
      <div className="image-viewer__toolbar">
        <span className={`stream-status stream-status--${status}`}>
          {status === "streaming" ? "Streaming" : status === "connecting" ? "Connecting…" : status === "error" ? "Error" : "Idle"}
        </span>
        {fpsDisplay && <span className="stream-fps">{fpsDisplay}</span>}
        {errorMsg && <span className="stream-error">{errorMsg}</span>}
        <span className="muted">
          {streamerInfo.width}×{streamerInfo.height}
        </span>
      </div>
      <div className="image-viewer__canvas-container">
        <canvas ref={canvasRef} className="image-viewer__canvas" />
      </div>
    </div>
  );
}
