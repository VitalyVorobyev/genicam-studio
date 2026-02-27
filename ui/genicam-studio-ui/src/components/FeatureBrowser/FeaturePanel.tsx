import { useMemo, useState } from "react";
import type { Diag, UiGraph, UiNode, UiNodeKind } from "../../xml_model/uigraph";
import type { NodeValue, ValueError } from "../../xml_model/values";
import type { NodeValueEntry } from "../../device/types";
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
  draftValue: NodeValue | undefined;
  draftErrors: ValueError[];
  hasDraft: boolean;
  onDraftChange: (value: NodeValue) => void;
  onDraftReset: () => void;
  canApply: boolean;
  applyDisabledReason: string;
  onApply: () => void;
  canExecute: boolean;
  executeDisabledReason: string;
  onExecute: () => void;
  /** Live value streamed from the connected device (undefined = no device or no data). */
  liveValue?: NodeValueEntry;
}

// Feature panel renders the selected node with a lightweight editor/view.
// Draft values live in a separate layer so UiGraph stays read-only.
export function FeaturePanel({
  graph,
  selectedNode,
  xmlText,
  diags,
  draftValue,
  draftErrors,
  hasDraft,
  onDraftChange,
  onDraftReset,
  canApply,
  applyDisabledReason,
  onApply,
  canExecute,
  executeDisabledReason,
  onExecute,
  liveValue,
}: FeaturePanelProps) {
  const [infoOpen, setInfoOpen] = useState(true);
  const [activeTab, setActiveTab] = useState<"raw" | "debug" | "diagnostics">(
    "raw"
  );

  const infoText = useMemo(() => {
    if (!selectedNode) {
      return null;
    }
    const info = [selectedNode.tooltip, selectedNode.comment, selectedNode.description]
      .filter(Boolean)
      .join("\n\n");
    return info || null;
  }, [selectedNode]);

  const diagnosticsPanel = renderDiagnostics(diags);
  const editable = selectedNode ? isEditableKind(selectedNode.kind) : false;
  const draftSummary = selectedNode && editable
    ? formatDraftSummary(selectedNode, draftValue, hasDraft)
    : null;

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

  const applyDisabled = !canApply || !hasDraft || draftErrors.length > 0;
  const applyTitle = applyDisabled
    ? applyDisabledReason || "Draft not ready."
    : "Apply draft to device.";

  return (
    <div className="feature-panel">
      <header className="feature-panel__header">
        <div>
          <h2>{nodeDisplayName(selectedNode)}</h2>
          <div className="feature-panel__meta">
            <span className="kind-badge">{nodeKindLabel(selectedNode.kind)}</span>
            <span className="muted">{selectedNode.name}</span>
            {liveValue !== undefined && (
              <span className="live-badge">
                Live: {String(liveValue.value)}
              </span>
            )}
            {(liveValue?.access_mode ?? selectedNode.access_mode) && (
              <span className="muted">
                access: {liveValue?.access_mode ?? selectedNode.access_mode}
              </span>
            )}
            {selectedNode.visibility && (
              <span className="muted">visibility: {selectedNode.visibility}</span>
            )}
          </div>
          {draftSummary && <div className="draft-summary">{draftSummary}</div>}
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

      <section className="feature-panel__body">
        {renderEditor(
          selectedNode,
          draftValue,
          draftErrors,
          onDraftChange,
          canExecute,
          executeDisabledReason,
          onExecute
        )}
      </section>

      {editable && (
        <section className="editor-actions">
          <button type="button" onClick={onDraftReset} disabled={!hasDraft}>
            Reset
          </button>
          <button type="button" onClick={onApply} disabled={applyDisabled} title={applyTitle}>
            Apply
          </button>
        </section>
      )}

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

function renderEditor(
  node: UiNode,
  draftValue: NodeValue | undefined,
  draftErrors: ValueError[],
  onDraftChange: (value: NodeValue) => void,
  canExecute: boolean,
  executeDisabledReason: string,
  onExecute: () => void
) {
  if (isUnknownKind(node.kind)) {
    return <UnknownDebugView raw={node.raw} />;
  }

  switch (node.kind) {
    case "Integer":
      return (
        <IntegerEditor
          node={node}
          value={draftValue}
          errors={draftErrors}
          onChange={onDraftChange}
        />
      );
    case "Float":
      return (
        <FloatEditor
          node={node}
          value={draftValue}
          errors={draftErrors}
          onChange={onDraftChange}
        />
      );
    case "Enumeration":
      return (
        <EnumEditor
          node={node}
          value={draftValue}
          errors={draftErrors}
          onChange={onDraftChange}
        />
      );
    case "Boolean":
      return (
        <BoolEditor
          value={draftValue}
          errors={draftErrors}
          onChange={onDraftChange}
        />
      );
    case "String":
      return (
        <StringEditor
          value={draftValue}
          errors={draftErrors}
          onChange={onDraftChange}
        />
      );
    case "Command":
      return (
        <CommandView
          canExecute={canExecute}
          onExecute={onExecute}
          disabledReason={executeDisabledReason}
        />
      );
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

function isEditableKind(kind: UiNodeKind): boolean {
  return (
    kind === "Integer" ||
    kind === "Float" ||
    kind === "Enumeration" ||
    kind === "Boolean" ||
    kind === "String"
  );
}

function formatDraftSummary(
  node: UiNode,
  value: NodeValue | undefined,
  hasDraft: boolean
) {
  if (!hasDraft || value === null || value === undefined) {
    return "Draft: unset (offline)";
  }

  if (node.kind === "Boolean" && typeof value === "boolean") {
    return `Draft: ${value ? "enabled" : "disabled"}`;
  }

  if (node.kind === "String" && typeof value === "string") {
    return value.length === 0 ? "Draft: \"\" (empty)" : `Draft: ${value}`;
  }

  if (node.kind === "Enumeration" && isEnumValue(value)) {
    const match = node.enum_entries?.find((entry) => entry.name === value.enumName);
    const label = match?.display_name ?? match?.name ?? value.enumName;
    return `Draft: ${label}`;
  }

  if (typeof value === "number") {
    return `Draft: ${value}`;
  }

  return "Draft: (unrecognized)";
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

function isEnumValue(value: NodeValue): value is { enumName: string } {
  return (
    typeof value === "object" &&
    value !== null &&
    "enumName" in value &&
    typeof (value as { enumName?: unknown }).enumName === "string"
  );
}
