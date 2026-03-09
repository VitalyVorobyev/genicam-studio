import { useEffect, useState } from "react";
import type { UiGraph } from "../../xml_model/uigraph";
import type { NodeValueEntry } from "../../device/types";
import type { VisibilityFilter } from "./FeatureBrowserPage";
import { CategoryTreeNode } from "./CategoryTreeNode";

interface CategoryTreeProps {
  graph: UiGraph | null;
  hideUnknown: boolean;
  visibilityFilter: VisibilityFilter;
  selectedNodeName: string | null;
  onSelectNode: (name: string) => void;
  liveValues?: Map<string, NodeValueEntry>;
}

export function CategoryTree({
  graph,
  hideUnknown,
  visibilityFilter,
  selectedNodeName,
  onSelectNode,
  liveValues,
}: CategoryTreeProps) {
  const rootCategory = graph?.root_category || "";
  const [expanded, setExpanded] = useState<Set<string>>(() => new Set());

  useEffect(() => {
    if (rootCategory && !expanded.has(rootCategory)) {
      setExpanded((prev) => new Set(prev).add(rootCategory));
    }
  }, [expanded, rootCategory]);

  if (!graph) {
    return <div className="tree-empty">Load an XML file to browse the feature tree.</div>;
  }

  if (!rootCategory) {
    return <div className="tree-empty">No root category found.</div>;
  }

  const toggleCategory = (name: string) => {
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(name)) next.delete(name);
      else next.add(name);
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
        visibilityFilter={visibilityFilter}
        selectedNodeName={selectedNodeName}
        expanded={expanded}
        onToggleCategory={toggleCategory}
        onSelectNode={onSelectNode}
        depth={0}
        liveValues={liveValues}
      />
    </div>
  );
}
