import { useEffect, useMemo, useState } from "react";
import type { UiNode } from "../../../xml_model/uigraph";

interface EnumEditorProps {
  node: UiNode;
}

// Enumeration editor uses local draft state (offline mode) and displays entries by
// display_name when available, falling back to the entry name.
export function EnumEditor({ node }: EnumEditorProps) {
  const entries = node.enum_entries ?? [];
  const options = useMemo(
    () =>
      entries.map((entry) => ({
        value: entry.value ?? entry.name,
        label: entry.display_name ?? entry.name,
      })),
    [entries]
  );

  const [draft, setDraft] = useState<string>("");

  useEffect(() => {
    if (options.length > 0) {
      setDraft(options[0].value);
    } else {
      setDraft("");
    }
  }, [options]);

  if (options.length === 0) {
    return <div className="editor__hint">No enum entries available.</div>;
  }

  return (
    <div className="editor">
      <label className="editor__label">Value</label>
      <select
        className="editor__input"
        value={draft}
        onChange={(event) => setDraft(event.target.value)}
      >
        {options.map((option) => (
          <option key={option.value} value={option.value}>
            {option.label}
          </option>
        ))}
      </select>
    </div>
  );
}
