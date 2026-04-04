import { useEffect, useState } from "react";
import type { UiGraph } from "../../xml_model/uigraph";
import type { NodeValueEntry } from "../../device/types";
import type { VisibilityFilter } from "./FeatureBrowserPage";
import { CategoryTreeNode } from "./CategoryTreeNode";
import { FavoriteLeafNode } from "./FavoriteLeafNode";

interface CategoryTreeProps {
  graph: UiGraph | null;
  hideUnknown: boolean;
  visibilityFilter: VisibilityFilter;
  selectedNodeName: string | null;
  onSelectNode: (name: string) => void;
  liveValues?: Map<string, NodeValueEntry>;
  favorites: Set<string>;
  onToggleFavorite: (name: string) => void;
}

export function CategoryTree({
  graph,
  hideUnknown,
  visibilityFilter,
  selectedNodeName,
  onSelectNode,
  liveValues,
  favorites,
  onToggleFavorite,
}: CategoryTreeProps) {
  const rootCategory = graph?.root_category || "";
  const [expanded, setExpanded] = useState<Set<string>>(() => new Set());
  const [favExpanded, setFavExpanded] = useState(true);

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

  // Filter favorites to only those present in the current graph
  const activeFavorites = [...favorites].filter(
    (name) => graph.nodes_by_name[name] !== undefined
  );

  return (
    <div className="category-tree">
      {activeFavorites.length > 0 && (
        <>
          <button
            type="button"
            className="tree-item tree-item--category tree-item--favorites-header"
            style={{ paddingLeft: "8px" }}
            onClick={() => setFavExpanded((prev) => !prev)}
          >
            <span className="tree-item__caret">{favExpanded ? "\u25BE" : "\u25B8"}</span>
            <span className="tree-item__icon tree-item__icon--favorites">{"\u2605"}</span>
            <span className="tree-item__label">Favorites</span>
            <span className="tree-item__meta">{activeFavorites.length}</span>
          </button>
          {favExpanded && (
            <ul className="tree-children">
              {activeFavorites.map((name) => (
                <li key={name}>
                  <FavoriteLeafNode
                    featureName={name}
                    graph={graph}
                    selectedNodeName={selectedNodeName}
                    onSelectNode={onSelectNode}
                    liveValues={liveValues}
                    isFavorited
                    onToggleFavorite={onToggleFavorite}
                  />
                </li>
              ))}
            </ul>
          )}
        </>
      )}
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
        favorites={favorites}
        onToggleFavorite={onToggleFavorite}
      />
    </div>
  );
}
