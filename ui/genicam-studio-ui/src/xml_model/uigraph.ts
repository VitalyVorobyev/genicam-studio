// This mirrors the Rust UiGraph JSON contract; keep in sync with fixtures/tests.
// No parsing logic here—TypeScript only consumes the JSON returned by WASM.

export interface UiGraph {
  nodes_by_name: Record<string, UiNode>;
  categories: Record<string, UiCategory>;
  root_category: string;
}

export interface UiNode {
  name: string;
  kind: UiNodeKind;
  display_name?: string;
  tooltip?: string;
  description?: string;
  visibility?: string;
  access_mode?: string;
  unit?: string;
  representation?: string;
  constraints?: NumericConstraints;
  enum_entries?: EnumEntry[];
  raw: RawNode;
}

export type UiNodeKind =
  | "Category"
  | "Integer"
  | "Float"
  | "Boolean"
  | "String"
  | "Enumeration"
  | "Command"
  | "Register"
  | { Unknown: { tag: string } };

export interface UiCategory {
  name: string;
  display_name: string;
  features: string[];
}

export interface NumericConstraints {
  min?: number;
  max?: number;
  inc?: number;
  value?: number;
}

export interface EnumEntry {
  name: string;
  value?: string;
  display_name?: string;
}

export interface RawNode {
  tag: string;
  attributes: Record<string, string>;
  children_text: Record<string, string>;
}
