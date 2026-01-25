import type { UiGraph } from "./uigraph";

export interface XmlModelProvider {
  parseXml(xml: string): Promise<any>;
}

// Browser-only provider that loads the Rust/WASM adapter on demand.
// Toolchain choice: wasm-pack generates an ES module + .wasm that Vite can import directly,
// keeping all parsing logic in Rust while TS only consumes the UiGraph JSON contract.
export class WebWasmProvider implements XmlModelProvider {
  async parseXml(xml: string): Promise<UiGraph> {
    const module = await loadWasmModule();
    return module.parse_xml_to_uigraph(xml) as UiGraph;
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
