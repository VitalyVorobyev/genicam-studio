import { useEffect, useState } from "react";
import type { UiNode } from "../../../xml_model/uigraph";

interface FloatEditorProps {
  node: UiNode;
}

// Float editor mirrors Integer editor but allows fractional input.
export function FloatEditor({ node }: FloatEditorProps) {
  const [draft, setDraft] = useState<string>("");

  useEffect(() => {
    const value = node.constraints?.value;
    setDraft(value !== undefined ? String(value) : "");
  }, [node]);

  return (
    <div className="editor">
      <label className="editor__label">Value</label>
      <input
        className="editor__input"
        type="number"
        step="any"
        value={draft}
        onChange={(event) => setDraft(event.target.value)}
      />
      <ConstraintDetails node={node} />
    </div>
  );
}

function ConstraintDetails({ node }: { node: UiNode }) {
  const { min, max, inc } = node.constraints ?? {};
  if (min === undefined && max === undefined && inc === undefined && !node.unit) {
    return null;
  }

  return (
    <div className="editor__constraints">
      {min !== undefined && <span>min: {min}</span>}
      {max !== undefined && <span>max: {max}</span>}
      {inc !== undefined && <span>inc: {inc}</span>}
      {node.unit && <span>unit: {node.unit}</span>}
    </div>
  );
}
