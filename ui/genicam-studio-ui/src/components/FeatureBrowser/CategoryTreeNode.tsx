import type { UiGraph, UiNode } from "../../xml_model/uigraph";
import {
  isUnknownKind,
  nodeDisplayName,
  nodeKindLabel,
  nodeKindCssKey,
  nodeKindIcon,
} from "../../xml_model/helpers";
import { visibilityPassesFilter, type VisibilityFilter } from "./FeatureBrowserPage";

interface CategoryTreeNodeProps {
  categoryName: string;
  graph: UiGraph;
  hideUnknown: boolean;
  visibilityFilter: VisibilityFilter;
  selectedNodeName: string | null;
  expanded: Set<string>;
  onToggleCategory: (name: string) => void;
  onSelectNode: (name: string) => void;
  depth: number;
  path?: Set<string>;
}

export function CategoryTreeNode({
  categoryName,
  graph,
  hideUnknown,
  visibilityFilter,
  selectedNodeName,
  expanded,
  onToggleCategory,
  onSelectNode,
  depth,
  path,
}: CategoryTreeNodeProps) {
  const category = graph.categories[categoryName];
  const isExpanded = expanded.has(categoryName);
  const indent = { paddingLeft: `${8 + depth * 16}px` };

  if (!category) {
    return (
      <div className="tree-item tree-item--missing" style={indent}>
        <span className="tree-item__label">Missing: {categoryName}</span>
      </div>
    );
  }

  const pathSet = path ?? new Set<string>();
  if (pathSet.has(categoryName)) {
    return (
      <div className="tree-item tree-item--missing" style={indent}>
        <span className="tree-item__label">Cycle: {categoryName}</span>
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
            ? "tree-item tree-item--category tree-item--active"
            : "tree-item tree-item--category"
        }
        style={indent}
        title={categoryTitle}
        onClick={() => {
          onToggleCategory(categoryName);
          onSelectNode(categoryName);
        }}
      >
        <span className="tree-item__caret">{isExpanded ? "▾" : "▸"}</span>
        <span className="tree-item__icon tree-item__icon--category">&#x25A6;</span>
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
                    style={{ paddingLeft: `${8 + (depth + 1) * 16}px` }}
                  >
                    <span className="tree-item__label">MissingRef</span>
                    <span className="tree-item__meta">{featureName}</span>
                  </div>
                </li>
              );
            }

            // T7.1 — apply visibility filter to leaf nodes
            if (!isCategoryNode(node) && hideUnknown && isUnknownKind(node.kind)) {
              return null;
            }

            if (!isCategoryNode(node) && !visibilityPassesFilter(node.visibility, visibilityFilter)) {
              return null;
            }

            if (isCategoryNode(node)) {
              return (
                <li key={featureName}>
                  <CategoryTreeNode
                    categoryName={featureName}
                    graph={graph}
                    hideUnknown={hideUnknown}
                    visibilityFilter={visibilityFilter}
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

            const nodeTitle = node.tooltip ?? node.comment ?? nodeDisplayName(node);
            const kindLabel = nodeKindLabel(node.kind);
            const kindKey = nodeKindCssKey(node.kind);
            const icon = nodeKindIcon(node.kind);

            return (
              <li key={featureName}>
                <button
                  type="button"
                  className={
                    featureName === selectedNodeName
                      ? "tree-item tree-item--active"
                      : "tree-item"
                  }
                  style={{ paddingLeft: `${8 + (depth + 1) * 16}px` }}
                  title={nodeTitle}
                  onClick={() => onSelectNode(featureName)}
                >
                  <span className="tree-item__caret" />
                  <span className={`tree-item__icon tree-item__icon--${kindKey}`}>
                    {icon}
                  </span>
                  <span className="tree-item__label">{nodeDisplayName(node)}</span>
                  <span className="tree-item__meta">{kindLabel}</span>
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
