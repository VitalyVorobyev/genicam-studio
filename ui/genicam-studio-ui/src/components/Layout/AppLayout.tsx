import { useCallback, useState } from "react";
import { isTauri } from "../../tauri";
import { useDevice } from "../../device/useDevice";
import { useNodeValues } from "../../device/useNodeValues";
import { useAcquisition } from "../../device/useAcquisition";
import { DeviceSidebar } from "../DeviceSidebar/DeviceSidebar";
import { FeatureBrowserPage } from "../FeatureBrowser/FeatureBrowserPage";
import { ImageViewer } from "../ImageViewer/ImageViewer";
import type { ParseXmlResponse } from "../../xml_model/uigraph";

type MainTab = "features" | "image" | "diagnostics";

export function AppLayout() {
  const [activeTab, setActiveTab] = useState<MainTab>("features");
  const [externalModel, setExternalModel] = useState<ParseXmlResponse | null>(null);
  const [connectError, setConnectError] = useState<string>("");

  const { devices, connectionState, connect, disconnect } = useDevice();
  const { liveValues } = useNodeValues();
  const { status: acqStatus, streamerInfo, start: startAcq, stop: stopAcq } = useAcquisition();

  const isConnected = connectionState.kind === "connected";
  const connectedDeviceName =
    connectionState.kind === "connected" ? connectionState.device_name : null;

  const handleConnect = useCallback(
    async (deviceId: string) => {
      setConnectError("");
      try {
        const response = await connect(deviceId);
        setExternalModel(response);
        setActiveTab("features");
      } catch (e) {
        const msg = typeof e === "object" && e !== null && "message" in e
          ? String((e as { message?: unknown }).message)
          : String(e);
        setConnectError(msg);
      }
    },
    [connect]
  );

  const handleStartAcquisition = useCallback(async () => {
    try {
      await startAcq();
      setActiveTab("image");
    } catch (e) {
      console.error("Start acquisition failed:", e);
    }
  }, [startAcq]);

  const windowTitle = connectedDeviceName
    ? `GenICam Studio — ${connectedDeviceName}`
    : "GenICam Studio";

  return (
    <div className="app-layout">
      <header className="app-header">
        <div className="app-header__brand">
          <h1>{windowTitle}</h1>
        </div>
        <div className="app-header__actions">
          {isConnected && !acqStatus.active && (
            <button type="button" onClick={handleStartAcquisition}>
              Start Acquisition
            </button>
          )}
          {isConnected && acqStatus.active && (
            <button type="button" className="btn--danger" onClick={() => stopAcq()}>
              Stop Acquisition
              {acqStatus.fps != null && ` (${acqStatus.fps.toFixed(1)} fps)`}
            </button>
          )}
        </div>
      </header>

      {connectError && (
        <div className="status status--error">{connectError}</div>
      )}

      <div className="app-body">
        {isTauri() && (
          <aside className="device-sidebar-wrapper">
            <DeviceSidebar
              devices={devices}
              connectionState={connectionState}
              onConnect={handleConnect}
              onDisconnect={disconnect}
            />
          </aside>
        )}

        <main className="main-area">
          <div className="main-tabs">
            <button
              type="button"
              className={activeTab === "features" ? "tab tab--active" : "tab"}
              onClick={() => setActiveTab("features")}
            >
              Feature Browser
            </button>
            <button
              type="button"
              className={activeTab === "image" ? "tab tab--active" : "tab"}
              onClick={() => setActiveTab("image")}
            >
              Image Viewer
              {acqStatus.active && <span className="tab__indicator"> ●</span>}
            </button>
            <button
              type="button"
              className={activeTab === "diagnostics" ? "tab tab--active" : "tab"}
              onClick={() => setActiveTab("diagnostics")}
            >
              Diagnostics
            </button>
          </div>

          <div className="main-content">
            <div style={{ display: activeTab === "features" ? "contents" : "none" }}>
              <FeatureBrowserPage
                externalModel={externalModel}
                liveValues={liveValues}
                isConnected={isConnected}
              />
            </div>
            {activeTab === "image" && <ImageViewer streamerInfo={streamerInfo} />}
            {activeTab === "diagnostics" && (
              <div className="diagnostics-placeholder">
                <p className="muted">Diagnostics tab — coming soon.</p>
              </div>
            )}
          </div>
        </main>
      </div>
    </div>
  );
}
