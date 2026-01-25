import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { ChangeEvent } from "react";
import type { Diag, ParseXmlResponse, UiGraph, UiNode } from "../../xml_model/uigraph";
import type { NodeValue } from "../../xml_model/values";
import { TauriProvider, WebWasmProvider } from "../../xml_model/provider";
import { isUnknownKind, nodeDisplayName } from "../../xml_model/helpers";
import { isTauri } from "../../tauri";
import { useDraftValues } from "../../state/useDraftValues";
import { CategoryTree } from "./CategoryTree";
import { FeaturePanel } from "./FeaturePanel";

type ParseStatus =
  | { kind: "idle" }
  | { kind: "loading"; fileName: string }
  | { kind: "error"; message: string }
  | { kind: "ready"; fileName: string };

// The Feature Browser is the main UI. It owns the loaded XML, UiGraph, and UI filters.
// Parsing happens only in Rust/WASM (via provider) and the UI only renders the JSON contract.
export function FeatureBrowserPage() {
  const fileInputRef = useRef<HTMLInputElement | null>(null);
  const provider = useMemo(
    () => (isTauri() ? new TauriProvider() : new WebWasmProvider()),
    []
  );

  const [graph, setGraph] = useState<UiGraph | null>(null);
  const [xmlText, setXmlText] = useState<string>("");
  const [selectedNodeName, setSelectedNodeName] = useState<string | null>(null);
  const [searchText, setSearchText] = useState("");
  const [hideUnknown, setHideUnknown] = useState(false);
  const [status, setStatus] = useState<ParseStatus>({ kind: "idle" });
  const [diags, setDiags] = useState<Diag[]>([]);
  const [summaryOverride, setSummaryOverride] = useState<string | null>(null);
  const [fixtures, setFixtures] = useState<string[]>([]);
  const [selectedFixture, setSelectedFixture] = useState<string>("");

  const { drafts, errors, setDraft, resetDraft, clearAllDrafts } = useDraftValues();

  const applyResponse = useCallback(
    (response: ParseXmlResponse, fileName: string) => {
      clearAllDrafts();
      setGraph(response.graph);
      setXmlText(response.xml);
      setSelectedNodeName(response.graph.root_category || null);
      setDiags(response.diags || []);
      setSummaryOverride(
        `${response.summary.node_count} nodes • ${response.summary.category_count} categories • root: ${response.summary.root_category}`
      );
      setStatus({ kind: "ready", fileName });
    },
    [clearAllDrafts]
  );

  useEffect(() => {
    let isMounted = true;

    if (provider.listFixtures) {
      provider
        .listFixtures()
        .then((names) => {
          if (!isMounted) {
            return;
          }
          setFixtures(names);
          setSelectedFixture(names[0] ?? "");
        })
        .catch(() => {
          if (isMounted) {
            setFixtures([]);
          }
        });
    }

    if (provider.getCurrentModel) {
      provider
        .getCurrentModel()
        .then((response) => {
          if (isMounted && response) {
            applyResponse(response, response.summary.root_category || "Current Model");
          }
        })
        .catch(() => {});
    }

    return () => {
      isMounted = false;
    };
  }, [applyResponse, provider]);

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
        const parsed = await provider.parseXml(xml);
        applyResponse(parsed, file.name);
      } catch (error) {
        setGraph(null);
        setXmlText("");
        setSelectedNodeName(null);
        setDiags([]);
        setSummaryOverride(null);
        setStatus({ kind: "error", message: formatErrorMessage(error) });
      } finally {
        event.target.value = "";
      }
    },
    [applyResponse, provider]
  );

  const onLoadFixture = useCallback(async () => {
    if (!provider.loadFixture || !selectedFixture) {
      return;
    }

    setStatus({ kind: "loading", fileName: selectedFixture });
    try {
      const parsed = await provider.loadFixture(selectedFixture);
      applyResponse(parsed, selectedFixture);
    } catch (error) {
      setStatus({ kind: "error", message: formatErrorMessage(error) });
    }
  }, [applyResponse, provider, selectedFixture]);

  const summary = useMemo(() => {
    if (summaryOverride) {
      return summaryOverride;
    }
    if (!graph) {
      return "No model loaded";
    }

    const nodeCount = Object.keys(graph.nodes_by_name ?? {}).length;
    const categoryCount = Object.keys(graph.categories ?? {}).length;
    const root = graph.root_category || "(none)";
    return `${nodeCount} nodes • ${categoryCount} categories • root: ${root}`;
  }, [graph, summaryOverride]);

  const selectedNode = useMemo<UiNode | null>(() => {
    if (!graph || !selectedNodeName) {
      return null;
    }
    return graph.nodes_by_name[selectedNodeName] ?? null;
  }, [graph, selectedNodeName]);

  const selectedDraftValue = selectedNode ? drafts[selectedNode.name] : undefined;
  const selectedDraftErrors = selectedNode ? errors[selectedNode.name] ?? [] : [];
  const selectedHasDraft = selectedNode
    ? Object.prototype.hasOwnProperty.call(drafts, selectedNode.name)
    : false;

  const onDraftChange = useCallback(
    (value: NodeValue) => {
      if (!selectedNode) {
        return;
      }
      setDraft(selectedNode, value);
    },
    [selectedNode, setDraft]
  );

  const onDraftReset = useCallback(() => {
    if (!selectedNode) {
      return;
    }
    resetDraft(selectedNode.name);
  }, [resetDraft, selectedNode]);

  const canApply = Boolean(provider.applyNodeValue);
  const canExecute = Boolean(provider.executeCommand);

  const applyDisabledReason = useMemo(() => {
    if (!provider.applyNodeValue) {
      return "Offline mode: will be enabled when connected to a device.";
    }
    if (!selectedHasDraft) {
      return "No draft value to apply.";
    }
    if (selectedDraftErrors.length > 0) {
      return "Resolve validation errors before applying.";
    }
    return "";
  }, [provider.applyNodeValue, selectedDraftErrors.length, selectedHasDraft]);

  const executeDisabledReason = useMemo(() => {
    if (!provider.executeCommand) {
      return "Offline mode: will be enabled when connected to a device.";
    }
    return "";
  }, [provider.executeCommand]);

  const onApply = useCallback(async () => {
    if (!provider.applyNodeValue || !selectedNode) {
      return;
    }
    if (!selectedHasDraft || selectedDraftErrors.length > 0) {
      return;
    }
    const value = drafts[selectedNode.name];
    if (value === undefined) {
      return;
    }
    await provider.applyNodeValue(selectedNode.name, value);
  }, [drafts, provider, selectedDraftErrors.length, selectedHasDraft, selectedNode]);

  const onExecute = useCallback(async () => {
    if (!provider.executeCommand || !selectedNode) {
      return;
    }
    await provider.executeCommand(selectedNode.name);
  }, [provider, selectedNode]);

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
          {fixtures.length > 0 && provider.loadFixture && (
            <div className="fixture-loader">
              <select
                value={selectedFixture}
                onChange={(event) => setSelectedFixture(event.target.value)}
              >
                {fixtures.map((fixture) => (
                  <option key={fixture} value={fixture}>
                    {fixture}
                  </option>
                ))}
              </select>
              <button type="button" onClick={onLoadFixture}>
                Load Fixture
              </button>
            </div>
          )}
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
        <div className="status status--info">Loading {status.fileName}...</div>
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
            diags={diags}
            draftValue={selectedDraftValue}
            draftErrors={selectedDraftErrors}
            hasDraft={selectedHasDraft}
            onDraftChange={onDraftChange}
            onDraftReset={onDraftReset}
            canApply={canApply}
            applyDisabledReason={applyDisabledReason}
            onApply={onApply}
            canExecute={canExecute}
            executeDisabledReason={executeDisabledReason}
            onExecute={onExecute}
          />
        </section>
      </div>
    </div>
  );
}

function formatErrorMessage(error: unknown) {
  if (typeof error === "string") {
    return error;
  }

  if (error instanceof Error) {
    return error.message;
  }

  if (error && typeof error === "object") {
    const message = (error as { message?: string }).message;
    const details = (error as { details?: string }).details;
    if (message && details) {
      return `${message} (${details})`;
    }
    if (message) {
      return message;
    }
    if (details) {
      return details;
    }
  }

  return String(error);
}
