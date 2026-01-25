import { useCallback, useEffect, useState } from "react";
import { isTauri, ping } from "./tauri";

export default function App() {
  const [debugOutput, setDebugOutput] = useState(
    "Ready. Click 'Load XML' to simulate an action."
  );

  const onLoadXml = useCallback(() => {
    setDebugOutput("Load XML clicked. (Wire XML parsing later.)");
  }, []);

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
        <button type="button" onClick={onLoadXml}>
          Load XML
        </button>
        <label className="debug-label" htmlFor="debug-output">
          Debug Output
        </label>
        <textarea id="debug-output" readOnly value={debugOutput} rows={10} />
      </main>
    </div>
  );
}
