import { useMemo } from "react";
import type { UiNode } from "../../../xml_model/uigraph";
import type { NodeValue, ValueError } from "../../../xml_model/values";
import { ValidationErrors } from "./ValidationErrors";

interface EnumEditorProps {
  node: UiNode;
  value: NodeValue | undefined;
  errors: ValueError[];
  onChange: (value: NodeValue) => void;
}

// Enumeration editor writes enum names into the shared draft store.
export function EnumEditor({ node, value, errors, onChange }: EnumEditorProps) {
  const entries = node.enum_entries ?? [];
  const options = useMemo(
    () =>
      entries.map((entry) => ({
        value: entry.name,
        label: entry.display_name ?? entry.name,
      })),
    [entries]
  );

  const selected = isEnumValue(value) ? value.enumName : "";
  const selectedLabel = useMemo(() => {
    if (!selected) {
      return null;
    }
    const match = options.find((option) => option.value === selected);
    return match?.label ?? selected;
  }, [options, selected]);

  if (options.length === 0) {
    return <div className="editor__hint">No enum entries available.</div>;
  }

  return (
    <div className="editor">
      <label className="editor__label">Value</label>
      <select
        className="editor__input"
        value={selected}
        onChange={(event) => {
          const next = event.target.value;
          if (!next) {
            onChange(null);
            return;
          }
          onChange({ enumName: next });
        }}
      >
        <option value="">(unset)</option>
        {options.map((option) => (
          <option key={option.value} value={option.value}>
            {option.label}
          </option>
        ))}
      </select>
      {selectedLabel && (
        <div className="editor__hint">Selected: {selectedLabel}</div>
      )}
      <ValidationErrors errors={errors} />
    </div>
  );
}

function isEnumValue(value: NodeValue | undefined): value is { enumName: string } {
  return (
    typeof value === "object" &&
    value !== null &&
    "enumName" in value &&
    typeof (value as { enumName?: unknown }).enumName === "string"
  );
}
