import type { UiNode } from "../../../xml_model/uigraph";
import type { NodeValue, ValueError } from "../../../xml_model/values";
import { ValidationErrors } from "./ValidationErrors";

interface FloatEditorProps {
  node: UiNode;
  value: NodeValue | undefined;
  errors: ValueError[];
  onChange: (value: NodeValue) => void;
}

// Float editor writes into the shared draft store (offline mode).
export function FloatEditor({ node, value, errors, onChange }: FloatEditorProps) {
  const numericValue = typeof value === "number" ? value : "";
  const { min, max, inc } = node.constraints ?? {};
  const step = inc ?? "any";

  return (
    <div className="editor">
      <label className="editor__label">Value</label>
      <div className="editor__input-row">
        <input
          className="editor__input"
          type="number"
          step={step}
          min={min}
          max={max}
          value={numericValue}
          placeholder="unset (offline)"
          onChange={(event) => {
            const raw = event.target.value;
            if (raw === "") {
              onChange(null);
              return;
            }
            const parsed = Number(raw);
            onChange(Number.isFinite(parsed) ? parsed : null);
          }}
        />
        {node.unit && <span className="editor__unit">{node.unit}</span>}
      </div>
      <ValidationErrors errors={errors} />
      <ConstraintDetails node={node} />
    </div>
  );
}

function ConstraintDetails({ node }: { node: UiNode }) {
  const { min, max, inc } = node.constraints ?? {};
  if (min === undefined && max === undefined && inc === undefined) {
    return null;
  }

  return (
    <div className="editor__constraints">
      {min !== undefined && <span>min: {min}</span>}
      {max !== undefined && <span>max: {max}</span>}
      {inc !== undefined && <span>inc: {inc}</span>}
    </div>
  );
}
