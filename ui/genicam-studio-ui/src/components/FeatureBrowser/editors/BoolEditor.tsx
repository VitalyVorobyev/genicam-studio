import { useEffect, useState } from "react";
import type { UiNode } from "../../../xml_model/uigraph";

interface BoolEditorProps {
  node: UiNode;
}

// Boolean editor uses local state; it does not persist back to the model.
export function BoolEditor({ node }: BoolEditorProps) {
  const [draft, setDraft] = useState(false);

  useEffect(() => {
    // Default to false; we don't assume a source-of-truth value in offline mode.
    setDraft(false);
  }, [node]);

  return (
    <label className="editor editor--inline">
      <input
        type="checkbox"
        checked={draft}
        onChange={(event) => setDraft(event.target.checked)}
      />
      Enabled
    </label>
  );
}
