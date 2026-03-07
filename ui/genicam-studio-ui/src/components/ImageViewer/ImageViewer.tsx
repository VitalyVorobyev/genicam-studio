import { useState } from "react";
import type { StreamerInfo } from "../../device/types";
import { ViewerToolbar } from "./ViewerToolbar";
import { ViewerCanvas } from "./ViewerCanvas";
import { ViewerStatusBar } from "./ViewerStatusBar";
import { ControlSidebar } from "./ControlSidebar";
import { useViewerLayout } from "./useViewerLayout";

interface ImageViewerProps {
  streamerInfo: StreamerInfo | null;
}

export function ImageViewer({ streamerInfo }: ImageViewerProps) {
  const [fps, setFps] = useState<number>(0);
  const { sidebarCollapsed, toggleSidebar } = useViewerLayout();

  if (!streamerInfo) {
    return (
      <div className="image-viewer image-viewer--idle">
        <p>Start acquisition to view the live image stream.</p>
      </div>
    );
  }

  return (
    <div className="image-viewer-v2">
      <div className="iv-canvas-column">
        <ViewerToolbar />
        <ViewerCanvas wsUrl={streamerInfo.ws_url} onFrameStats={setFps} />
        <ViewerStatusBar
          fps={fps}
          width={streamerInfo.width}
          height={streamerInfo.height}
          pixelFormat=""
        />
      </div>
      <ControlSidebar
        collapsed={sidebarCollapsed}
        onToggle={toggleSidebar}
      />
    </div>
  );
}
