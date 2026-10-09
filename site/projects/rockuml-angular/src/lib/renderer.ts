import { Injectable, inject } from '@angular/core';
import type { Format, RenderOptions, Rendering, Rockuml } from 'rockuml';

import { ROCKUML_LOADER } from './provider';

/** Renders diagram sources with the one rockuml engine of the application, loading it on first use. */
@Injectable({ providedIn: 'root' })
export class RockumlRenderer {
  private readonly loader = inject(ROCKUML_LOADER);
  private engine?: Promise<Rockuml>;

  render(
    source: string,
    options: RenderOptions & { format: 'png' },
  ): Promise<Rendering<Uint8Array>>;
  render(
    source: string,
    options?: RenderOptions & { format?: Exclude<Format, 'png'> },
  ): Promise<Rendering<string>>;
  render(source: string, options?: RenderOptions): Promise<Rendering>;
  async render(source: string, options?: RenderOptions): Promise<Rendering> {
    return (await this.loadEngine()).render(source, options);
  }

  private loadEngine(): Promise<Rockuml> {
    // A failed download is tried again by the next rendering instead of failing every diagram for good.
    this.engine ??= this.loader().catch((error: unknown) => {
      this.engine = undefined;
      throw error;
    });
    return this.engine;
  }
}
