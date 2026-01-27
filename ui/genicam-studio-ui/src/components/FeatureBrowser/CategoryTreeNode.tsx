import type { UiGraph, UiNode } from "../../xml_model/uigraph";
import { isUnknownKind, nodeDisplayName, nodeKindLabel } from "../../xml_model/helpers";

interface CategoryTreeNodeProps {
  categoryName: string;
  graph: UiGraph;
  hideUnknown: boolean;
  selectedNodeName: string | null;
  expanded: Set<string>;
  onToggleCategory: (name: string) => void;
  onSelectNode: (name: string) => void;
  depth: number;
  path?: Set<string>;
}

// Recursive renderer that looks up nodes by name on demand.
// Avoids building a full tree structure while still supporting nested categories.
export function CategoryTreeNode({
  categoryName,
  graph,
  hideUnknown,
  selectedNodeName,
  expanded,
  onToggleCategory,
  onSelectNode,
  depth,
  path,
}: CategoryTreeNodeProps) {
  const category = graph.categories[categoryName];
  const isExpanded = expanded.has(categoryName);

  const padding = { paddingLeft: `${depth * 16}px` };

  if (!category) {
    return (
      <div className="tree-item tree-item--missing" style={padding}>
        <span className="tree-item__label">Missing category: {categoryName}</span>
      </div>
    );
  }

  const pathSet = path ?? new Set();
  if (pathSet.has(categoryName)) {
    // Guard against accidental cycles in category references.
    return (
      <div className="tree-item tree-item--missing" style={padding}>
        <span className="tree-item__label">Cycle detected: {categoryName}</span>
      </div>
    );
  }

  const nextPath = new Set(pathSet);
  nextPath.add(categoryName);
  const categoryTitle = category.tooltip ?? category.comment ?? category.display_name;

  return (
    <div className="tree-node">
      <button
        type="button"
        className={
          categoryName === selectedNodeName
            ? "tree-item tree-item--active"
            : "tree-item"
        }
        style={padding}
        title={categoryTitle}
        onClick={() => {
          onToggleCategory(categoryName);
          onSelectNode(categoryName);
        }}
      >
        <span className="tree-item__caret">{isExpanded ? "▾" : "▸"}</span>
        <span className="tree-item__label">{category.display_name}</span>
        <span className="tree-item__meta">{category.name}</span>
      </button>

      {isExpanded && (
        <ul className="tree-children">
          {category.features.map((featureName) => {
            const node = graph.nodes_by_name[featureName];
            if (!node) {
              return (
                <li key={featureName}>
                  <div
                    className="tree-item tree-item--missing"
                    style={{ paddingLeft: `${(depth + 1) * 16}px` }}
                  >
                    <span className="tree-item__label">MissingRef</span>
                    <span className="tree-item__meta">{featureName}</span>
                  </div>
                </li>
              );
            }

            if (hideUnknown && isUnknownKind(node.kind)) {
              return null;
            }

            if (isCategoryNode(node)) {
              return (
                <li key={featureName}>
                  <CategoryTreeNode
                    categoryName={featureName}
                    graph={graph}
                    hideUnknown={hideUnknown}
                    selectedNodeName={selectedNodeName}
                    expanded={expanded}
                    onToggleCategory={onToggleCategory}
                    onSelectNode={onSelectNode}
                    depth={depth + 1}
                    path={nextPath}
                  />
                </li>
              );
            }

            return (
              <li key={featureName}>
                <button
                  type="button"
                  className={
                    featureName === selectedNodeName
                      ? "tree-item tree-item--active"
                      : "tree-item"
                  }
                  style={{ paddingLeft: `${(depth + 1) * 16}px` }}
                  title={node.tooltip ?? node.comment ?? nodeDisplayName(node)}
                  onClick={() => onSelectNode(featureName)}
                >
                  <span className="tree-item__label">
                    {nodeDisplayName(node)}
                  </span>
                  <span className="tree-item__meta">{nodeKindLabel(node.kind)}</span>
                </button>
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}

function isCategoryNode(node: UiNode): boolean {
  return typeof node.kind === "string" && node.kind === "Category";
}
