import { useCallback, useEffect, useState } from "react";
import { isTauri } from "../tauri";
import type { AcquisitionStatus, StreamerInfo } from "./types";

export function useAcquisition() {
  const [status, setStatus] = useState<AcquisitionStatus>({
    active: false,
    fps: null,
    dropped: 0,
  });
  const [streamerInfo, setStreamerInfo] = useState<StreamerInfo | null>(null);

  useEffect(() => {
    if (!isTauri()) return;

    let unlisten: (() => void) | null = null;
    let cancelled = false;

    import("@tauri-apps/api/event").then(({ listen }) => {
      if (cancelled) return;
      listen<AcquisitionStatus>("acquisition-status", (e) => {
        setStatus(e.payload);
        if (!e.payload.active) {
          setStreamerInfo(null);
        }
      }).then((fn) => {
        if (cancelled) fn();
        else unlisten = fn;
      });
    });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  const start = useCallback(async (): Promise<StreamerInfo> => {
    const { invoke } = await import("@tauri-apps/api/core");
    const info = await invoke<StreamerInfo>("start_acquisition");
    setStreamerInfo(info);
    return info;
  }, []);

  const stop = useCallback(async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    await invoke("stop_acquisition");
    setStreamerInfo(null);
  }, []);

  return { status, streamerInfo, start, stop };
}
