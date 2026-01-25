import { useEffect, useState } from "react";
import type { UiGraph } from "../../xml_model/uigraph";
import { CategoryTreeNode } from "./CategoryTreeNode";

interface CategoryTreeProps {
  graph: UiGraph | null;
  hideUnknown: boolean;
  selectedNodeName: string | null;
  onSelectNode: (name: string) => void;
}

// The tree renders categories on demand from the UiGraph contract.
// We intentionally avoid building a full tree model to keep data structures minimal.
export function CategoryTree({
  graph,
  hideUnknown,
  selectedNodeName,
  onSelectNode,
}: CategoryTreeProps) {
  const rootCategory = graph?.root_category || "";
  const [expanded, setExpanded] = useState<Set<string>>(() => new Set());

  // Ensure the root is expanded on first render for a loaded graph.
  useEffect(() => {
    if (rootCategory && !expanded.has(rootCategory)) {
      setExpanded((prev) => new Set(prev).add(rootCategory));
    }
  }, [expanded, rootCategory]);

  if (!graph) {
    return <div className="tree-empty">Load an XML file to browse categories.</div>;
  }

  if (!rootCategory) {
    return <div className="tree-empty">No root category found.</div>;
  }

  const toggleCategory = (name: string) => {
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(name)) {
        next.delete(name);
      } else {
        next.add(name);
      }
      return next;
    });
  };

  return (
    <div className="category-tree">
      <div className="category-tree__header">Categories</div>
      <CategoryTreeNode
        categoryName={rootCategory}
        graph={graph}
        hideUnknown={hideUnknown}
        selectedNodeName={selectedNodeName}
        expanded={expanded}
        onToggleCategory={toggleCategory}
        onSelectNode={onSelectNode}
        depth={0}
      />
    </div>
  );
}
