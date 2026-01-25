import { useCallback, useEffect, useRef, useState } from "react";
import { isTauri, ping } from "./tauri";
import { WebWasmProvider } from "./xml_model/provider";
import type { UiGraph } from "./xml_model/uigraph";

const xmlProvider = new WebWasmProvider();

export default function App() {
  const fileInputRef = useRef<HTMLInputElement | null>(null);
  const [debugOutput, setDebugOutput] = useState(
    "Ready. Click 'Load XML' to select a file."
  );

  const onLoadXml = useCallback(() => {
    fileInputRef.current?.click();
  }, []);

  const onFileSelected = useCallback(
    async (event: React.ChangeEvent<HTMLInputElement>) => {
      const file = event.target.files?.[0];
      if (!file) {
        return;
      }

      setDebugOutput(`Reading ${file.name}...`);

      try {
        const xml = await file.text();
        const graph = await xmlProvider.parseXml(xml);
        setDebugOutput(buildSummary(graph, file.name));
      } catch (error) {
        setDebugOutput(`Parse failed: ${String(error)}`);
      } finally {
        event.target.value = "";
      }
    },
    []
  );

  useEffect(() => {
    if (!isTauri()) {
      setDebugOutput((prev) => `${prev}\nRunning in browser mode.`);
      return;
    }

    ping()
      .then((response) => {
        setDebugOutput((prev) => `${prev}\nTauri ping() -> ${response}`);
      })
      .catch((error) => {
        setDebugOutput((prev) => `${prev}\nTauri ping() failed: ${String(error)}`);
      });
  }, []);

  return (
    <div className="app">
      <header>
        <h1>GenICam Studio</h1>
        <p>XML tools for GenICam devices.</p>
      </header>
      <main>
        <input
          ref={fileInputRef}
          type="file"
          accept=".xml,text/xml"
          onChange={onFileSelected}
          hidden
        />
        <button type="button" onClick={onLoadXml}>
          Load XML
        </button>
        <label className="debug-label" htmlFor="debug-output">
          Debug Output
        </label>
        <textarea id="debug-output" readOnly value={debugOutput} rows={12} />
      </main>
    </div>
  );
}

function buildSummary(graph: UiGraph, fileName: string): string {
  const nodeCount = Object.keys(graph.nodes_by_name ?? {}).length;
  const categoryCount = Object.keys(graph.categories ?? {}).length;
  const root = graph.root_category || "(none)";

  return [
    `Parsed ${fileName}.`,
    `Root category: ${root}`,
    `Nodes: ${nodeCount}`,
    `Categories: ${categoryCount}`,
  ].join("\n");
}
