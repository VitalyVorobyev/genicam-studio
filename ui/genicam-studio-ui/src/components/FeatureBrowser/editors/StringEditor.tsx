import { useEffect, useState } from "react";
import type { UiNode } from "../../../xml_model/uigraph";

interface StringEditorProps {
  node: UiNode;
}

// String editor uses a local draft value for offline preview.
export function StringEditor({ node }: StringEditorProps) {
  const [draft, setDraft] = useState("");

  useEffect(() => {
    setDraft("");
  }, [node]);

  return (
    <div className="editor">
      <label className="editor__label">Value</label>
      <input
        className="editor__input"
        type="text"
        value={draft}
        onChange={(event) => setDraft(event.target.value)}
      />
    </div>
  );
}
