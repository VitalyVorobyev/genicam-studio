import type { ParseXmlResponse, UiGraph } from "./uigraph";
import type { NodeValue } from "./values";

export interface XmlModelProvider {
  parseXml(xml: string): Promise<ParseXmlResponse>;
  listFixtures?(): Promise<string[]>;
  loadFixture?(name: string): Promise<ParseXmlResponse>;
  getCurrentModel?(): Promise<ParseXmlResponse | null>;
  applyNodeValue?(nodeName: string, value: NodeValue): Promise<void>;
  executeCommand?(nodeName: string): Promise<void>;
}

// Tauri provider bridges the UI to native Rust parsing. It also exposes fixtures
// and can fetch the last loaded model from the Rust-side state.
export class TauriProvider implements XmlModelProvider {
  async parseXml(xml: string): Promise<ParseXmlResponse> {
    return await invokeNative<ParseXmlResponse>("parse_xml", { xml });
  }

  async listFixtures(): Promise<string[]> {
    return await invokeNative<string[]>("list_fixtures");
  }

  async loadFixture(name: string): Promise<ParseXmlResponse> {
    return await invokeNative<ParseXmlResponse>("load_fixture", { name });
  }

  async getCurrentModel(): Promise<ParseXmlResponse | null> {
    return await invokeNative<ParseXmlResponse | null>("get_current_model");
  }

  async applyNodeValue(nodeName: string, value: NodeValue): Promise<void> {
    await invokeNative("write_node", { nodeName, value: nodeValueToJson(value) });
  }

  async executeCommand(nodeName: string): Promise<void> {
    await invokeNative("execute_command", { nodeName });
  }
}

async function invokeNative<T>(command: string, payload?: Record<string, unknown>) {
  const { invoke } = await import("@tauri-apps/api/core");
  return (await invoke(command, payload)) as T;
}

function buildSummary(graph: UiGraph) {
  return {
    node_count: Object.keys(graph.nodes_by_name ?? {}).length,
    category_count: Object.keys(graph.categories ?? {}).length,
    root_category: graph.root_category || "",
  };
}

function nodeValueToJson(value: NodeValue): unknown {
  if (value === null) return null;
  if (typeof value === "object" && "enumName" in value) return value.enumName;
  return value;
}
