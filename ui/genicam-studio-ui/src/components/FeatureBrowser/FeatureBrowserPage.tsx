import { useCallback, useMemo, useRef, useState } from "react";
import type { ChangeEvent } from "react";
import type { UiGraph, UiNode } from "../../xml_model/uigraph";
import { WebWasmProvider } from "../../xml_model/provider";
import { isUnknownKind, nodeDisplayName } from "../../xml_model/helpers";
import { CategoryTree } from "./CategoryTree";
import { FeaturePanel } from "./FeaturePanel";

const xmlProvider = new WebWasmProvider();

type ParseStatus =
  | { kind: "idle" }
  | { kind: "loading"; fileName: string }
  | { kind: "error"; message: string }
  | { kind: "ready"; fileName: string };

// The Feature Browser is the main UI. It owns the loaded XML, UiGraph, and UI filters.
// Parsing happens only in Rust/WASM (via provider) and the UI only renders the JSON contract.
export function FeatureBrowserPage() {
  const fileInputRef = useRef<HTMLInputElement | null>(null);
  const [graph, setGraph] = useState<UiGraph | null>(null);
  const [xmlText, setXmlText] = useState<string>("");
  const [selectedNodeName, setSelectedNodeName] = useState<string | null>(null);
  const [searchText, setSearchText] = useState("");
  const [hideUnknown, setHideUnknown] = useState(false);
  const [status, setStatus] = useState<ParseStatus>({ kind: "idle" });

  const onLoadXml = useCallback(() => {
    fileInputRef.current?.click();
  }, []);

  const onFileSelected = useCallback(
    async (event: ChangeEvent<HTMLInputElement>) => {
      const file = event.target.files?.[0];
      if (!file) {
        return;
      }

      setStatus({ kind: "loading", fileName: file.name });

      try {
        const xml = await file.text();
        const parsed = await xmlProvider.parseXml(xml);
        setGraph(parsed as UiGraph);
        setXmlText(xml);
        setSelectedNodeName(parsed.root_category || null);
        setStatus({ kind: "ready", fileName: file.name });
      } catch (error) {
        setGraph(null);
        setXmlText("");
        setSelectedNodeName(null);
        setStatus({ kind: "error", message: String(error) });
      } finally {
        event.target.value = "";
      }
    },
    []
  );

  const summary = useMemo(() => {
    if (!graph) {
      return "No model loaded";
    }

    const nodeCount = Object.keys(graph.nodes_by_name ?? {}).length;
    const categoryCount = Object.keys(graph.categories ?? {}).length;
    const root = graph.root_category || "(none)";
    return `${nodeCount} nodes • ${categoryCount} categories • root: ${root}`;
  }, [graph]);

  const selectedNode = useMemo<UiNode | null>(() => {
    if (!graph || !selectedNodeName) {
      return null;
    }
    return graph.nodes_by_name[selectedNodeName] ?? null;
  }, [graph, selectedNodeName]);

  const searchResults = useMemo(() => {
    if (!graph || !searchText.trim()) {
      return [] as UiNode[];
    }

    const query = searchText.trim().toLowerCase();
    return Object.values(graph.nodes_by_name)
      .filter((node) => {
        if (hideUnknown && isUnknownKind(node.kind)) {
          return false;
        }
        const display = nodeDisplayName(node).toLowerCase();
        return node.name.toLowerCase().includes(query) || display.includes(query);
      })
      .slice(0, 200);
  }, [graph, hideUnknown, searchText]);

  return (
    <div className="app feature-browser">
      <header className="top-bar">
        <div className="top-bar__left">
          <h1>GenICam Studio</h1>
          <p>XML tools for GenICam devices.</p>
        </div>
        <div className="top-bar__controls">
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
          <input
            className="search-input"
            type="search"
            placeholder="Search name or display name"
            value={searchText}
            onChange={(event) => setSearchText(event.target.value)}
          />
          <label className="toggle">
            <input
              type="checkbox"
              checked={hideUnknown}
              onChange={(event) => setHideUnknown(event.target.checked)}
            />
            Hide Unknown
          </label>
          <span className="summary-badge" title={summary}>
            {summary}
          </span>
        </div>
      </header>

      {status.kind === "loading" && (
        <div className="status status--info">
          Loading {status.fileName}...
        </div>
      )}
      {status.kind === "error" && (
        <div className="status status--error">{status.message}</div>
      )}
      {status.kind === "ready" && (
        <div className="status status--success">Loaded {status.fileName}</div>
      )}

      <div className="feature-browser__body">
        <aside className="pane pane--left">
          <div className="pane__section">
            {searchText.trim().length > 0 && (
              <div className="search-results">
                <div className="search-results__header">Search Results</div>
                {searchResults.length === 0 ? (
                  <div className="search-results__empty">No matches.</div>
                ) : (
                  <ul>
                    {searchResults.map((node) => (
                      <li key={node.name}>
                        <button
                          type="button"
                          className={
                            node.name === selectedNodeName
                              ? "tree-item tree-item--active"
                              : "tree-item"
                          }
                          onClick={() => setSelectedNodeName(node.name)}
                        >
                          <span className="tree-item__label">
                            {nodeDisplayName(node)}
                          </span>
                          <span className="tree-item__meta">{node.name}</span>
                        </button>
                      </li>
                    ))}
                  </ul>
                )}
              </div>
            )}
            <CategoryTree
              graph={graph}
              hideUnknown={hideUnknown}
              selectedNodeName={selectedNodeName}
              onSelectNode={setSelectedNodeName}
            />
          </div>
        </aside>

        <section className="pane pane--right">
          <FeaturePanel
            graph={graph}
            selectedNode={selectedNode}
            xmlText={xmlText}
          />
        </section>
      </div>
    </div>
  );
}
