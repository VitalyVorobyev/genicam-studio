import { useState, useRef, useEffect } from "react";
import type { StreamerInfo } from "../../device/types";
import type { ParseXmlResponse, EnumEntry } from "../../xml_model/uigraph";
import { ViewerToolbar } from "./ViewerToolbar";
import { ViewerCanvas } from "./ViewerCanvas";
import { ViewerStatusBar } from "./ViewerStatusBar";
import { ControlSidebar } from "./ControlSidebar";
import { useViewerLayout } from "./useViewerLayout";

interface ImageViewerProps {
  streamerInfo: StreamerInfo | null;
  isConnected: boolean;
  isAcquiring: boolean;
  onStartAcq: () => Promise<void>;
  onStopAcq: () => Promise<void>;
  externalModel: ParseXmlResponse | null;
}

export function ImageViewer({
  streamerInfo,
  isConnected,
  isAcquiring,
  onStartAcq,
  onStopAcq,
  externalModel,
}: ImageViewerProps) {
  const [fps, setFps] = useState<number>(0);
  const [frameCount, setFrameCount] = useState<number>(0);
  const prevAcquiring = useRef<boolean>(false);
  const { sidebarCollapsed, toggleSidebar } = useViewerLayout();

  // Reset frame counter each time acquisition transitions false → true
  useEffect(() => {
    if (isAcquiring && !prevAcquiring.current) {
      setFrameCount(0);
    }
    prevAcquiring.current = isAcquiring;
  }, [isAcquiring]);

  const acquisitionModeEntries: EnumEntry[] =
    externalModel?.graph.nodes_by_name["AcquisitionMode"]?.enum_entries ?? [];

  if (!streamerInfo) {
    return (
      <div className="image-viewer-v2">
        <div className="iv-canvas-column">
          <ViewerToolbar />
          <div className="iv-canvas-wrap image-viewer--idle">
            <p>Start acquisition to view the live image stream.</p>
          </div>
        </div>
        <ControlSidebar
          collapsed={sidebarCollapsed}
          onToggle={toggleSidebar}
          isConnected={isConnected}
          isAcquiring={isAcquiring}
          frameCount={frameCount}
          onStartAcq={onStartAcq}
          onStopAcq={onStopAcq}
          acquisitionModeEntries={acquisitionModeEntries}
        />
      </div>
    );
  }

  return (
    <div className="image-viewer-v2">
      <div className="iv-canvas-column">
        <ViewerToolbar />
        <ViewerCanvas
          wsUrl={streamerInfo.ws_url}
          onFrameStats={setFps}
          onFrame={() => setFrameCount((c) => c + 1)}
        />
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
        isConnected={isConnected}
        isAcquiring={isAcquiring}
        frameCount={frameCount}
        onStartAcq={onStartAcq}
        onStopAcq={onStopAcq}
        acquisitionModeEntries={acquisitionModeEntries}
      />
    </div>
  );
}
