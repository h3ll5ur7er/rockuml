// rockuml in the browser and in Node: renders PlantUML sources with rockuml.wasm.
//
//   import { load } from './rockuml.js';
//   const rockuml = await load();                      // or load(url), load(bytes), load(WebAssembly.Module)
//   const { data, pageCount, isError } = await rockuml.render(source, { format: 'svg', page: 0 });
//
// `data` is a string for the text formats and a Uint8Array for PNG. Sources rockuml cannot render throw a
// RockumlError whose `notPorted` says whether the diagram needs a part of PlantUML not ported yet.

const FORMATS = { svg: 0, png: 1, 'svg-deterministic': 2, debug: 3, preproc: 4 };
const BINARY_FORMATS = new Set(['png']);

const STATUS_ERROR_IMAGE = 1;
const STATUS_NOT_PORTED = 2;
const STATUS_NO_IMAGE = 3;

export class RockumlError extends Error {
  constructor(message, { notPorted, pageCount }) {
    super(message);
    this.name = 'RockumlError';
    this.notPorted = notPorted;
    this.pageCount = pageCount;
  }
}

export const formats = Object.keys(FORMATS);

export async function load(wasm = new URL('rockuml.wasm', import.meta.url)) {
  const module = await compile(wasm);
  let instance = instantiate(module);
  return {
    async render(source, { format = 'svg', page = 0 } = {}) {
      if (!(format in FORMATS)) {
        throw new TypeError(`unknown format ${format}; rockuml renders ${formats.join(', ')}`);
      }
      const { exports } = await instance;
      try {
        return render(exports, source, format, page);
      } catch (error) {
        // A trap (a panic in rockuml) or a stack overflow leaves the instance's stack and state behind.
        if (!(error instanceof RockumlError)) {
          instance = instantiate(module);
        }
        throw error;
      }
    },
  };
}

async function compile(wasm) {
  if (wasm instanceof WebAssembly.Module) {
    return wasm;
  }
  if (wasm instanceof ArrayBuffer || ArrayBuffer.isView(wasm)) {
    return WebAssembly.compile(wasm);
  }
  const response = await fetch(wasm);
  if (!response.ok) {
    throw new Error(`cannot load ${wasm}: ${response.status} ${response.statusText}`);
  }
  return WebAssembly.compile(await response.arrayBuffer());
}

async function instantiate(module) {
  let memory;
  const instance = await WebAssembly.instantiate(module, {
    rockuml: {
      current_time_millis: () => Date.now(),
      local_time_zone(pointer, capacity) {
        const name = Intl.DateTimeFormat().resolvedOptions().timeZone ?? '';
        const { read, written } = new TextEncoder().encodeInto(
          name,
          new Uint8Array(memory.buffer, pointer, capacity),
        );
        return read === name.length ? written : 0;
      },
    },
  });
  memory = instance.exports.memory;
  return instance;
}

function render(exports, source, format, page) {
  const bytes = new TextEncoder().encode(source);
  const pointer = exports.rockuml_alloc(bytes.length);
  let status;
  try {
    new Uint8Array(exports.memory.buffer, pointer, bytes.length).set(bytes);
    status = exports.rockuml_render(pointer, bytes.length, FORMATS[format], page);
  } finally {
    exports.rockuml_free(pointer, bytes.length);
  }
  const output = new Uint8Array(
    exports.memory.buffer,
    exports.rockuml_output_pointer(),
    exports.rockuml_output_length(),
  ).slice();
  const pageCount = exports.rockuml_page_count();
  if (status === STATUS_NOT_PORTED || status === STATUS_NO_IMAGE) {
    throw new RockumlError(new TextDecoder().decode(output), {
      notPorted: status === STATUS_NOT_PORTED,
      pageCount,
    });
  }
  return {
    data: BINARY_FORMATS.has(format) ? output : new TextDecoder().decode(output),
    pageCount,
    isError: status === STATUS_ERROR_IMAGE,
  };
}
