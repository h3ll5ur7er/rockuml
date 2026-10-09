// The types of rockuml.js.

/** What `render` makes: images, the debug description PlantUML's tests use, or the preprocessed source. */
export type Format = 'svg' | 'png' | 'svg-deterministic' | 'debug' | 'preproc';

export const formats: readonly Format[];

export interface RenderOptions {
  /** `svg` unless given. */
  format?: Format;
  /** The page to render, counted over all diagrams of the source; 0 unless given. */
  page?: number;
}

export interface Rendering<Data = string | Uint8Array> {
  /** A string, or the bytes of a PNG. */
  data: Data;
  /** The pages of all the source's diagrams together. */
  pageCount: number;
  /** The image shows the diagram's errors instead of the diagram. */
  isError: boolean;
}

/** The source has no diagram, page or format to render, or needs a part of PlantUML rockuml does not have yet. */
export class RockumlError extends Error {
  constructor(message: string, details: { notPorted: boolean; pageCount: number });
  readonly name: 'RockumlError';
  /** The source needs a part of PlantUML rockuml does not have yet. */
  readonly notPorted: boolean;
  readonly pageCount: number;
}

export interface Rockuml {
  render(source: string, options: RenderOptions & { format: 'png' }): Promise<Rendering<Uint8Array>>;
  render(
    source: string,
    options?: RenderOptions & { format?: Exclude<Format, 'png'> },
  ): Promise<Rendering<string>>;
  render(source: string, options?: RenderOptions): Promise<Rendering>;
}

/** Compiles rockuml.wasm: from next to rockuml.js unless a URL, its bytes or a compiled module is given. */
export function load(wasm?: string | URL | ArrayBuffer | ArrayBufferView | WebAssembly.Module): Promise<Rockuml>;
