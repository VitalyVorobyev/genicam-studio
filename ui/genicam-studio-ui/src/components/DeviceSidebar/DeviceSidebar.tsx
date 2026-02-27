import type { ConnectionState, DeviceInfo } from "../../device/types";
import { DeviceCard } from "./DeviceCard";

interface DeviceSidebarProps {
  devices: DeviceInfo[];
  connectionState: ConnectionState;
  onConnect: (deviceId: string) => void;
  onDisconnect: () => void;
}

export function DeviceSidebar({
  devices,
  connectionState,
  onConnect,
  onDisconnect,
}: DeviceSidebarProps) {
  const statusLabel = connectionStateLabel(connectionState);
  const statusClass = connectionStateClass(connectionState);

  return (
    <div className="device-sidebar">
      <div className="device-sidebar__header">
        <span className="device-sidebar__title">Devices</span>
        <span className={`conn-badge conn-badge--${statusClass}`}>{statusLabel}</span>
      </div>

      {connectionState.kind === "error" && (
        <div className="device-sidebar__error">{connectionState.message}</div>
      )}

      <div className="device-sidebar__list">
        {devices.length === 0 ? (
          <div className="device-sidebar__empty">Scanning for devices…</div>
        ) : (
          devices.map((device) => (
            <DeviceCard
              key={device.id}
              device={device}
              connectionState={connectionState}
              onConnect={onConnect}
              onDisconnect={onDisconnect}
            />
          ))
        )}
      </div>
    </div>
  );
}

function connectionStateLabel(state: ConnectionState): string {
  switch (state.kind) {
    case "disconnected":
      return "Disconnected";
    case "connecting":
      return "Connecting…";
    case "connected":
      return `Connected`;
    case "error":
      return "Error";
  }
}

function connectionStateClass(state: ConnectionState): string {
  switch (state.kind) {
    case "disconnected":
      return "disconnected";
    case "connecting":
      return "connecting";
    case "connected":
      return "connected";
    case "error":
      return "error";
  }
}
