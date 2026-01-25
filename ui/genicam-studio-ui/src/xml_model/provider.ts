import type { ParseXmlResponse, UiGraph } from "./uigraph";
import type { NodeValue } from "./values";

export interface XmlModelProvider {
  parseXml(xml: string): Promise<ParseXmlResponse>;
  listFixtures?(): Promise<string[]>;
  loadFixture?(name: string): Promise<ParseXmlResponse>;
  getCurrentModel?(): Promise<ParseXmlResponse | null>;
  // Optional live actions for future device integration.
  applyNodeValue?(nodeName: string, value: NodeValue): Promise<void>;
  executeCommand?(nodeName: string): Promise<void>;
}

// Browser-only provider that loads the Rust/WASM adapter on demand.
// Toolchain choice: wasm-pack generates an ES module + .wasm that Vite can import directly,
// keeping all parsing logic in Rust while TS only consumes the UiGraph JSON contract.
export class WebWasmProvider implements XmlModelProvider {
  async parseXml(xml: string): Promise<ParseXmlResponse> {
    const module = await loadWasmModule();
    const graph = module.parse_xml_to_uigraph(xml) as UiGraph;
    return {
      graph,
      xml,
      diags: [],
      summary: buildSummary(graph),
    };
  }
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
}

type WasmModule = {
  default: () => Promise<void>;
  parse_xml_to_uigraph: (xml: string) => unknown;
  version: () => string;
};

let wasmModulePromise: Promise<WasmModule> | null = null;

async function loadWasmModule(): Promise<WasmModule> {
  if (!wasmModulePromise) {
    wasmModulePromise = import(
      "../wasm/genicam_xml_model_wasm/genicam_xml_model_wasm.js"
    )
      .then(async (module) => {
        const typed = module as WasmModule;
        await typed.default();
        return typed;
      })
      .catch((error) => {
        const message =
          "Failed to load WASM parser. Run `npm run wasm:build` first. " +
          String(error);
        throw new Error(message);
      });
  }

  return wasmModulePromise;
}

async function invokeNative<T>(command: string, payload?: Record<string, unknown>) {
  // Dynamic import keeps the browser bundle from hard-depending on Tauri APIs.
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
