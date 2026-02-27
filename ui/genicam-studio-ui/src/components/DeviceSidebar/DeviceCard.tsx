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
  const isError =
    connectionState.kind === "error";
  const isBusy =
    connectionState.kind === "connecting" || connectionState.kind === "connected";

  return (
    <div className={`device-card ${isConnected ? "device-card--connected" : ""} ${isError && isConnecting ? "device-card--error" : ""}`}>
      <div className="device-card__info">
        <div className="device-card__name">{device.name}</div>
        <div className="device-card__meta">
          <span>{device.model}</span>
          <span className="muted">{device.serial}</span>
        </div>
        <div className="device-card__id muted">{device.id}</div>
      </div>
      <div className="device-card__actions">
        {isConnected ? (
          <button type="button" className="btn--secondary" onClick={onDisconnect}>
            Disconnect
          </button>
        ) : (
          <button
            type="button"
            disabled={isBusy || isConnecting}
            onClick={() => onConnect(device.id)}
          >
            {isConnecting ? "Connecting…" : "Connect"}
          </button>
        )}
      </div>
    </div>
  );
}
