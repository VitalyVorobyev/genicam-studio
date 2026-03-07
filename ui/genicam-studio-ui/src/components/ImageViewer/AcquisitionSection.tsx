import { useState } from "react";
import type { EnumEntry } from "../../xml_model/uigraph";
import { isTauri } from "../../tauri";
import { formatFrameCount } from "./viewerUtils";

interface AcquisitionSectionProps {
  isConnected: boolean;
  isAcquiring: boolean;
  frameCount: number;
  onStartAcq: () => Promise<void>;
  onStopAcq: () => Promise<void>;
  acquisitionModeEntries: EnumEntry[];
}

export function AcquisitionSection({
  isConnected,
  isAcquiring,
  frameCount,
  onStartAcq,
  onStopAcq,
  acquisitionModeEntries,
}: AcquisitionSectionProps) {
  const [busy, setBusy] = useState(false);

  if (!isConnected) {
    return <p className="sidebar-placeholder">No device connected.</p>;
  }

  const handleToggle = async () => {
    if (busy) return;
    setBusy(true);
    try {
      if (isAcquiring) {
        await onStopAcq();
      } else {
        await onStartAcq();
      }
    } finally {
      setBusy(false);
    }
  };

  const handleModeChange = async (e: React.ChangeEvent<HTMLSelectElement>) => {
    const newMode = e.target.value;
    if (!isTauri()) return;
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("write_node", { nodeName: "AcquisitionMode", value: newMode });
    } catch (err) {
      console.error("AcquisitionMode write failed:", err);
    }
  };

  const frameDisplay =
    !isAcquiring && frameCount === 0 ? "—" : formatFrameCount(frameCount);

  return (
    <div className="acq-section">
      <button
        type="button"
        className={isAcquiring ? "acq-btn acq-btn--stop" : "acq-btn"}
        disabled={busy}
        onClick={handleToggle}
      >
        {busy
          ? isAcquiring
            ? "Stopping…"
            : "Starting…"
          : isAcquiring
          ? "Stop Acquisition"
          : "Start Acquisition"}
      </button>

      {acquisitionModeEntries.length > 0 && (
        <div className="acq-mode">
          <label className="editor__label" htmlFor="acq-mode-select">
            Acquisition Mode
          </label>
          <select
            id="acq-mode-select"
            disabled={isAcquiring}
            onChange={handleModeChange}
          >
            {acquisitionModeEntries.map((entry) => (
              <option key={entry.name} value={entry.name}>
                {entry.display_name ?? entry.name}
              </option>
            ))}
          </select>
        </div>
      )}

      <div className="frame-counter">
        <span className="editor__label">Frames received</span>
        <span className="frame-counter__value">{frameDisplay}</span>
      </div>
    </div>
  );
}
