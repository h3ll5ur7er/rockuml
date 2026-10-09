import { EnvironmentProviders, InjectionToken, makeEnvironmentProviders } from '@angular/core';
import { load, type Rockuml } from 'rockuml';

/** Loads the rockuml engine. The renderer calls it once, when the first diagram is rendered. */
export type RockumlLoader = () => Promise<Rockuml>;

export const ROCKUML_LOADER = new InjectionToken<RockumlLoader>(
  'ROCKUML_LOADER: provide it with provideRockuml()',
);

export interface RockumlConfig {
  /** Where `rockuml.wasm` is served; a relative URL is relative to the page's base URL. */
  wasm: string | URL;
}

/**
 * Makes rockuml render the application's diagrams. Bundlers move `rockuml.js` away from `rockuml.wasm`, so the
 * application says where it serves the wasm module, for example by copying it among its assets.
 */
export function provideRockuml(config: RockumlConfig): EnvironmentProviders {
  return makeEnvironmentProviders([
    { provide: ROCKUML_LOADER, useValue: (() => load(config.wasm)) satisfies RockumlLoader },
  ]);
}
