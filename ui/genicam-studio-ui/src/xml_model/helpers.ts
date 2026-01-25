import type { UiNode, UiNodeKind } from "./uigraph";

// Keep presentation helpers tiny and pure so we don't invent parallel models.
export function nodeDisplayName(node: UiNode): string {
  return node.display_name?.trim() || node.name;
}

export function nodeKindLabel(kind: UiNodeKind): string {
  if (typeof kind === "string") {
    return kind;
  }

  return `Unknown (${kind.Unknown.tag})`;
}

export function isUnknownKind(kind: UiNodeKind): boolean {
  return typeof kind !== "string";
}
