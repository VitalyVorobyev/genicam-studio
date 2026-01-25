import { useMemo, useState } from "react";
import type { Diag, UiGraph, UiNode } from "../../xml_model/uigraph";
import { isUnknownKind, nodeDisplayName, nodeKindLabel } from "../../xml_model/helpers";
import { BoolEditor } from "./editors/BoolEditor";
import { CommandView } from "./editors/CommandView";
import { EnumEditor } from "./editors/EnumEditor";
import { FloatEditor } from "./editors/FloatEditor";
import { IntegerEditor } from "./editors/IntegerEditor";
import { StringEditor } from "./editors/StringEditor";
import { UnknownDebugView } from "./editors/UnknownDebugView";

interface FeaturePanelProps {
  graph: UiGraph | null;
  selectedNode: UiNode | null;
  xmlText: string;
  diags: Diag[];
}

// Feature panel renders the selected node with a lightweight editor/view.
// All edits are local drafts (offline), so we don't mutate the UiGraph contract.
export function FeaturePanel({ graph, selectedNode, xmlText, diags }: FeaturePanelProps) {
  const [infoOpen, setInfoOpen] = useState(true);
  const [activeTab, setActiveTab] = useState<"raw" | "debug" | "diagnostics">(
    "raw"
  );

  const infoText = useMemo(() => {
    if (!selectedNode) {
      return null;
    }
    const info = [selectedNode.tooltip, selectedNode.description]
      .filter(Boolean)
      .join("\n\n");
    return info || null;
  }, [selectedNode]);

  const diagnosticsPanel = renderDiagnostics(diags);

  if (!graph || !selectedNode) {
    return (
      <div className="feature-panel feature-panel--empty">
        <p>Select a feature to view details.</p>
        <div className="tabs">
          <button
            type="button"
            className={activeTab === "raw" ? "tab tab--active" : "tab"}
            onClick={() => setActiveTab("raw")}
          >
            Raw XML
          </button>
          <button
            type="button"
            className={activeTab === "debug" ? "tab tab--active" : "tab"}
            onClick={() => setActiveTab("debug")}
          >
            Model Debug
          </button>
          <button
            type="button"
            className={
              activeTab === "diagnostics" ? "tab tab--active" : "tab"
            }
            onClick={() => setActiveTab("diagnostics")}
          >
            Diagnostics
          </button>
        </div>
        <div className="tab-panel">
          {activeTab === "raw" ? (
            <textarea readOnly value={xmlText || "(no XML loaded)"} />
          ) : activeTab === "debug" ? (
            <pre>(no selection)</pre>
          ) : (
            diagnosticsPanel
          )}
        </div>
      </div>
    );
  }

  return (
    <div className="feature-panel">
      <header className="feature-panel__header">
        <div>
          <h2>{nodeDisplayName(selectedNode)}</h2>
          <div className="feature-panel__meta">
            <span className="kind-badge">{nodeKindLabel(selectedNode.kind)}</span>
            <span className="muted">{selectedNode.name}</span>
            {selectedNode.access_mode && (
              <span className="muted">access: {selectedNode.access_mode}</span>
            )}
            {selectedNode.visibility && (
              <span className="muted">visibility: {selectedNode.visibility}</span>
            )}
          </div>
        </div>
      </header>

      {infoText && (
        <section className="info">
          <button
            type="button"
            className="info__toggle"
            onClick={() => setInfoOpen((prev) => !prev)}
          >
            {infoOpen ? "Hide Info" : "Show Info"}
          </button>
          {infoOpen && <pre className="info__content">{infoText}</pre>}
        </section>
      )}

      <section className="feature-panel__body">{renderEditor(selectedNode)}</section>

      <section className="feature-panel__tabs">
        <div className="tabs">
          <button
            type="button"
            className={activeTab === "raw" ? "tab tab--active" : "tab"}
            onClick={() => setActiveTab("raw")}
          >
            Raw XML
          </button>
          <button
            type="button"
            className={activeTab === "debug" ? "tab tab--active" : "tab"}
            onClick={() => setActiveTab("debug")}
          >
            Model Debug
          </button>
          <button
            type="button"
            className={
              activeTab === "diagnostics" ? "tab tab--active" : "tab"
            }
            onClick={() => setActiveTab("diagnostics")}
          >
            Diagnostics
          </button>
        </div>
        <div className="tab-panel">
          {activeTab === "raw" ? (
            <textarea readOnly value={xmlText || "(no XML loaded)"} />
          ) : activeTab === "debug" ? (
            <pre>{JSON.stringify(selectedNode, null, 2)}</pre>
          ) : (
            diagnosticsPanel
          )}
        </div>
      </section>
    </div>
  );
}

function renderEditor(node: UiNode) {
  if (isUnknownKind(node.kind)) {
    return <UnknownDebugView raw={node.raw} />;
  }

  switch (node.kind) {
    case "Integer":
      return <IntegerEditor node={node} />;
    case "Float":
      return <FloatEditor node={node} />;
    case "Enumeration":
      return <EnumEditor node={node} />;
    case "Boolean":
      return <BoolEditor node={node} />;
    case "String":
      return <StringEditor node={node} />;
    case "Command":
      return <CommandView />;
    case "Register":
      return (
        <div className="editor__hint">
          Register node (not editable in offline mode).
        </div>
      );
    case "Category":
      return (
        <div className="editor__hint">
          Category node — select a feature in the tree to edit.
        </div>
      );
    default:
      return null;
  }
}

// Diagnostics are read-only hints from the parser; keep rendering lightweight.
function renderDiagnostics(diags: Diag[]) {
  if (!diags || diags.length === 0) {
    return <div className="diagnostics diagnostics--empty">No diagnostics.</div>;
  }

  return (
    <ul className="diagnostics">
      {diags.map((diag, index) => (
        <li key={`${diag.level}-${diag.message}-${index}`}>
          <span className={`diagnostics__level diagnostics__level--${diag.level}`}>
            {diag.level.toUpperCase()}
          </span>
          <span className="diagnostics__message">{diag.message}</span>
          {diag.node && <span className="diagnostics__node">({diag.node})</span>}
        </li>
      ))}
    </ul>
  );
}
