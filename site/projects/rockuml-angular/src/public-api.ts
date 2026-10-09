// rockuml for Angular: diagrams rendered in the browser by rockuml's WebAssembly engine. The editors are in
// @rockuml/angular/editor.

export * from './lib/types';
export {
  provideRockuml,
  ROCKUML_LOADER,
  type RockumlConfig,
  type RockumlLoader,
} from './lib/provider';
export { RockumlRenderer } from './lib/renderer';
export { RockumlDiagram } from './lib/diagram';
export { encodeSource, decodeSource } from './lib/source-code';
