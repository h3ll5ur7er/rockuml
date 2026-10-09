import { TestBed } from '@angular/core/testing';
import type { Rockuml } from 'rockuml';

import { ROCKUML_LOADER } from './provider';
import { RockumlRenderer } from './renderer';

const svgEngine: Rockuml = {
  render: async (source: string) => ({
    data: `<svg>${source}</svg>`,
    pageCount: 1,
    isError: false,
  }),
} as Rockuml;

function rendererLoadingWith(loader: () => Promise<Rockuml>): RockumlRenderer {
  TestBed.configureTestingModule({ providers: [{ provide: ROCKUML_LOADER, useValue: loader }] });
  return TestBed.inject(RockumlRenderer);
}

describe('RockumlRenderer', () => {
  it('loads the engine once for all renderings', async () => {
    const loader = vi.fn(async () => svgEngine);
    const renderer = rendererLoadingWith(loader);

    expect((await renderer.render('A')).data).toBe('<svg>A</svg>');
    expect((await renderer.render('B')).data).toBe('<svg>B</svg>');
    expect(loader).toHaveBeenCalledTimes(1);
  });

  it('loads the engine again after a failed download', async () => {
    const loader = vi
      .fn<() => Promise<Rockuml>>()
      .mockRejectedValueOnce(new Error('offline'))
      .mockResolvedValue(svgEngine);
    const renderer = rendererLoadingWith(loader);

    await expect(renderer.render('A')).rejects.toThrow('offline');
    expect((await renderer.render('A')).data).toBe('<svg>A</svg>');
  });
});
