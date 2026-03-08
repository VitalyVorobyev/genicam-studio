import { useCallback, useEffect, useState } from "react";
import { isTauri } from "../../tauri";
import { useDevice } from "../../device/useDevice";
import { useNodeValues } from "../../device/useNodeValues";
import { useAcquisition } from "../../device/useAcquisition";
import { useImageMeta } from "../../device/useImageMeta";
import { AppLogProvider, useAppLog } from "../../context/AppLogContext";
import { DeviceSidebar } from "../DeviceSidebar/DeviceSidebar";
import { FeatureBrowserPage } from "../FeatureBrowser/FeatureBrowserPage";
import { ImageViewer } from "../ImageViewer/ImageViewer";
import { DiagnosticsTab } from "../Diagnostics/DiagnosticsTab";
import { formatDeviceChip } from "./headerUtils";
import type { ParseXmlResponse } from "../../xml_model/uigraph";

type MainTab = "features" | "image" | "diagnostics";

// AppLayout is wrapped by AppLogProvider so all children can call useAppLog.
export function AppLayout() {
  return (
    <AppLogProvider>
      <AppLayoutInner />
    </AppLogProvider>
  );
}

function AppLayoutInner() {
  const [activeTab, setActiveTab] = useState<MainTab>("features");
  const [externalModel, setExternalModel] = useState<ParseXmlResponse | null>(null);
  const [connectError, setConnectError] = useState<string>("");

  const { devices, connectionState, connect, disconnect } = useDevice();
  const { liveValues } = useNodeValues();
  const { status: acqStatus, streamerInfo, start: startAcq, stop: stopAcq } = useAcquisition();
  const { imageMeta } = useImageMeta();
  const { log } = useAppLog();

  const isConnected = connectionState.kind === "connected";
  const connectedDeviceName =
    connectionState.kind === "connected" ? connectionState.device_name : null;

  // T7.7 — keep document.title in sync with connection state
  useEffect(() => {
    document.title = connectedDeviceName
      ? `GenICam Studio — ${connectedDeviceName}`
      : "GenICam Studio";
  }, [connectedDeviceName]);

  const handleConnect = useCallback(
    async (deviceId: string) => {
      setConnectError("");
      log("info", "Connecting to device\u2026", deviceId);
      try {
        const response = await connect(deviceId);
        setExternalModel(response);
        setActiveTab("features");
        log("success", `Connected`, deviceId);
      } catch (e) {
        const msg =
          typeof e === "object" && e !== null && "message" in e
            ? String((e as { message?: unknown }).message)
            : String(e);
        setConnectError(msg);
        log("error", `Connection failed: ${msg}`, deviceId);
      }
    },
    [connect, log]
  );

  const handleDisconnect = useCallback(async () => {
    const name = connectedDeviceName ?? "device";
    try {
      await disconnect();
      log("info", `Disconnected from ${name}`);
    } catch (e) {
      log("error", `Disconnect error: ${String(e)}`);
    }
  }, [disconnect, connectedDeviceName, log]);

  const handleStartAcquisition = useCallback(async () => {
    log("info", "Starting acquisition\u2026");
    try {
      await startAcq();
      setActiveTab("image");
      log("success", "Acquisition started");
    } catch (e) {
      log("error", `Acquisition start failed: ${String(e)}`);
    }
  }, [startAcq, log]);

  const handleStopAcquisition = useCallback(async () => {
    log("info", "Stopping acquisition\u2026");
    try {
      await stopAcq();
      log("success", "Acquisition stopped");
    } catch (e) {
      log("error", `Acquisition stop failed: ${String(e)}`);
    }
  }, [stopAcq, log]);

  const chip = formatDeviceChip(connectionState);

  return (
    <div className="app-layout">
      <header className="app-header">
        {/* Left: brand wordmark */}
        <div className="app-header__brand">
          <h1 className="app-header__brand-name">GenICam Studio</h1>
        </div>

        {/* Center: tab navigation */}
        <nav className="app-header__tabs">
          <button
            type="button"
            className={
              activeTab === "features"
                ? "app-header__tab app-header__tab--active"
                : "app-header__tab"
            }
            onClick={() => setActiveTab("features")}
          >
            Feature Browser
          </button>
          <button
            type="button"
            className={
              activeTab === "image"
                ? "app-header__tab app-header__tab--active"
                : "app-header__tab"
            }
            onClick={() => setActiveTab("image")}
          >
            Image Viewer
          </button>
          <button
            type="button"
            className={
              activeTab === "diagnostics"
                ? "app-header__tab app-header__tab--active"
                : "app-header__tab"
            }
            onClick={() => setActiveTab("diagnostics")}
          >
            Diagnostics
          </button>
        </nav>

        {/* Right: acquisition indicator + device chip */}
        <div className="app-header__actions">
          {isConnected && !acqStatus.active && (
            <button type="button" className="btn" onClick={handleStartAcquisition}>
              Start Acquisition
            </button>
          )}
          {acqStatus.active && (
            <div className="app-header__acq-indicator">
              <span className="app-header__acq-dot" />
              Acquiring{acqStatus.fps != null ? ` ${acqStatus.fps.toFixed(1)} fps` : ""}
            </div>
          )}
          {isConnected && acqStatus.active && (
            <button
              type="button"
              className="btn--stop-sm"
              onClick={handleStopAcquisition}
            >
              Stop
            </button>
          )}
          <div className="app-header__device-chip">
            <span
              className={`app-header__device-dot app-header__device-dot--${chip.state}`}
            />
            {chip.label}
          </div>
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
              onDisconnect={handleDisconnect}
            />
          </aside>
        )}

        <main className="main-area">
          <div className="main-content">
            {/* Feature Browser stays mounted so tree state is preserved between tabs */}
            <div style={{ display: activeTab === "features" ? "contents" : "none" }}>
              <FeatureBrowserPage
                externalModel={externalModel}
                liveValues={liveValues}
                isConnected={isConnected}
              />
            </div>
            {activeTab === "image" && (
              <ImageViewer
                streamerInfo={streamerInfo}
                isConnected={isConnected}
                isAcquiring={acqStatus.active}
                onStartAcq={handleStartAcquisition}
                onStopAcq={handleStopAcquisition}
                externalModel={externalModel}
                liveValues={liveValues}
                deviceName={connectedDeviceName ?? undefined}
                imageMeta={imageMeta}
              />
            )}
            {activeTab === "diagnostics" && <DiagnosticsTab />}
          </div>
        </main>
      </div>
    </div>
  );
}
