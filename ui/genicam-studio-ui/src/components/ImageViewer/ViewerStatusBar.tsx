import { formatFps, formatResolution } from "./viewerUtils";

interface ViewerStatusBarProps {
  fps: number;
  width: number;
  height: number;
  pixelFormat: string;
}

export function ViewerStatusBar({
  fps,
  width,
  height,
  pixelFormat,
}: ViewerStatusBarProps) {
  const resolution = formatResolution(width, height);
  const fpsText = formatFps(fps);

  return (
    <div className="iv-statusbar">
      <span>{resolution}</span>
      {pixelFormat !== "" && (
        <>
          <span className="iv-statusbar__sep">·</span>
          <span>{pixelFormat}</span>
        </>
      )}
      <span className="iv-statusbar__sep">·</span>
      <span>{fpsText}</span>
    </div>
  );
}
