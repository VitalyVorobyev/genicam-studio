import { useEffect, useState } from "react";
import { isTauri } from "../tauri";
import type { NodeValueEntry } from "./types";

export function useNodeValues() {
  const [liveValues, setLiveValues] = useState<Map<string, NodeValueEntry>>(new Map());

  useEffect(() => {
    if (!isTauri()) return;

    let unlisten: (() => void) | null = null;
    let cancelled = false;

    import("@tauri-apps/api/event").then(({ listen }) => {
      if (cancelled) return;
      listen<{ node_name: string; value: number | string | boolean; access_mode: string }>(
        "node-value-changed",
        (event) => {
          const { node_name, value, access_mode } = event.payload;
          setLiveValues((prev) => {
            const next = new Map(prev);
            next.set(node_name, { value, access_mode });
            return next;
          });
        }
      ).then((fn) => {
        if (cancelled) {
          fn();
        } else {
          unlisten = fn;
        }
      });
    });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  // Clear cache when device disconnects
  useEffect(() => {
    if (!isTauri()) return;

    let unlisten: (() => void) | null = null;
    import("@tauri-apps/api/event").then(({ listen }) => {
      listen("connection-state-changed", (e) => {
        const state = e.payload as { kind: string };
        if (state.kind === "disconnected" || state.kind === "error") {
          setLiveValues(new Map());
        }
      }).then((fn) => {
        unlisten = fn;
      });
    });

    return () => {
      unlisten?.();
    };
  }, []);

  return { liveValues };
}
