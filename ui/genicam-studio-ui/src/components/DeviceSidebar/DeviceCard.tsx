import type { DeviceInfo, ConnectionState } from "../../device/types";

interface DeviceCardProps {
  device: DeviceInfo;
  connectionState: ConnectionState;
  onConnect: (id: string) => void;
  onDisconnect: () => void;
}

export function DeviceCard({ device, connectionState, onConnect, onDisconnect }: DeviceCardProps) {
  const isConnecting =
    connectionState.kind === "connecting" &&
    connectionState.device_id === device.id;
  const isConnected =
    connectionState.kind === "connected" &&
    connectionState.device_id === device.id;
  const isBusy =
    connectionState.kind === "connecting" || connectionState.kind === "connected";

  return (
    <div className={`device-card${isConnected ? " device-card--connected" : ""}`}>
      <div className="device-card__name">{device.name}</div>
      <div className="device-card__meta">
        {device.model && <span>{device.model}</span>}
        {device.serial && <span className="muted">{device.serial}</span>}
      </div>
      <div className="device-card__id">{device.id}</div>
      <div className="device-card__actions">
        {isConnected ? (
          <button type="button" className="btn--secondary" onClick={onDisconnect}>
            Disconnect
          </button>
        ) : (
          <button
            type="button"
            className="btn"
            disabled={isBusy || isConnecting}
            onClick={() => onConnect(device.id)}
          >
            {isConnecting ? "Connecting\u2026" : "Connect"}
          </button>
        )}
      </div>
    </div>
  );
}
