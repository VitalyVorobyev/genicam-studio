import { useState, useRef, useEffect, useCallback } from "react";
import type { StreamerInfo, NodeValueEntry, ImageMeta } from "../../device/types";
import type { ParseXmlResponse, EnumEntry } from "../../xml_model/uigraph";
import type { ZoomPanState } from "./useZoomPan";
import { ViewerToolbar } from "./ViewerToolbar";
import { ViewerCanvas } from "./ViewerCanvas";
import type { PixelHoverInfo } from "./ViewerCanvas";
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
  liveValues: Map<string, NodeValueEntry>;
  deviceName?: string;
  cameraModel?: string;
  imageMeta: ImageMeta | null;
}

export function ImageViewer({
  streamerInfo,
  isConnected,
  isAcquiring,
  onStartAcq,
  onStopAcq,
  externalModel,
  liveValues,
  deviceName,
  cameraModel,
  imageMeta,
}: ImageViewerProps) {
  const [fps, setFps] = useState<number>(0);
  const [frameCount, setFrameCount] = useState<number>(0);
  const [zoomLabel, setZoomLabel] = useState<string>("Fit");
  const [pixelHover, setPixelHover] = useState<PixelHoverInfo | null>(null);
  const prevAcquiring = useRef<boolean>(false);
  const { sidebarCollapsed, toggleSidebar } = useViewerLayout();
  const resetZoomRef = useRef<(() => void) | null>(null);

  // Reset frame counter each time acquisition transitions false → true
  useEffect(() => {
    if (isAcquiring && !prevAcquiring.current) {
      setFrameCount(0);
    }
    prevAcquiring.current = isAcquiring;
  }, [isAcquiring]);

  const handleZoomPanChange = (s: ZoomPanState) => {
    setZoomLabel(s.zoomLabel);
  };

  const handleResetZoom = useCallback(() => {
    resetZoomRef.current?.();
  }, []);

  const acquisitionModeEntries: EnumEntry[] =
    externalModel?.graph.nodes_by_name["AcquisitionMode"]?.enum_entries ?? [];

  const pixelFormat = imageMeta?.pixel_format ?? "Mono8";

  if (!streamerInfo) {
    return (
      <div className="image-viewer-v2">
        <div className="iv-canvas-column">
          <ViewerToolbar deviceName={deviceName} onResetZoom={handleResetZoom} />
          <div className="iv-canvas-wrap iv-canvas-wrap--idle">
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
          liveValues={liveValues}
          externalModel={externalModel}
          cameraModel={cameraModel ?? null}
        />
      </div>
    );
  }

  return (
    <div className="image-viewer-v2">
      <div className="iv-canvas-column">
        <ViewerToolbar zoomLabel={zoomLabel} deviceName={deviceName} onResetZoom={handleResetZoom} />
        <ViewerCanvas
          wsUrl={streamerInfo.ws_url}
          onFrameStats={setFps}
          onFrame={() => setFrameCount((c) => c + 1)}
          onZoomPanChange={handleZoomPanChange}
          pixelFormat={pixelFormat}
          onPixelHover={setPixelHover}
          resetZoomRef={resetZoomRef}
          isStreaming={true}
        />
        <ViewerStatusBar
          fps={fps}
          width={imageMeta?.width ?? streamerInfo.width}
          height={imageMeta?.height ?? streamerInfo.height}
          pixelFormat={pixelFormat}
          pixelInfo={
            pixelHover
              ? { coords: pixelHover.coords, formatted: pixelHover.formatted }
              : null
          }
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
        liveValues={liveValues}
        externalModel={externalModel}
        cameraModel={cameraModel ?? null}
      />
    </div>
  );
}
