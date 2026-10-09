import { Component, signal } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { RockumlError, type Rendering } from '@rockuml/core';

import { RockumlDiagram } from './diagram';
import { RockumlRenderer } from './renderer';

@Component({
  imports: [RockumlDiagram],
  template: `<rockuml-diagram [source]="source()" (rendered)="renderings.push($event)" />`,
})
class Host {
  readonly source = signal('A -> B');
  readonly renderings: Rendering<string>[] = [];
}

/** A renderer whose renderings finish when the test says so. */
class ControlledRenderer {
  readonly calls: { source: string; finish: (result: Rendering | Error) => void }[] = [];

  render(source: string): Promise<Rendering> {
    return new Promise((resolve, reject) =>
      this.calls.push({
        source,
        finish: (result) => (result instanceof Error ? reject(result) : resolve(result)),
      }),
    );
  }
}

const svg = (text: string): Rendering<string> => ({
  data: `<svg>${text}</svg>`,
  pageCount: 1,
  isError: false,
});

async function setUp() {
  let blobs = 0;
  vi.spyOn(URL, 'createObjectURL').mockImplementation(() => `blob:image-${++blobs}`);
  vi.spyOn(URL, 'revokeObjectURL').mockImplementation(() => undefined);
  const renderer = new ControlledRenderer();
  TestBed.configureTestingModule({ providers: [{ provide: RockumlRenderer, useValue: renderer }] });
  const fixture = TestBed.createComponent(Host);
  await fixture.whenStable();
  await vi.waitFor(() => expect(renderer.calls.length).toBeGreaterThan(0));
  const settle = async () => {
    await new Promise((resolve) => setTimeout(resolve));
    await fixture.whenStable();
  };
  return { fixture, renderer, settle, element: fixture.nativeElement as HTMLElement };
}

describe('RockumlDiagram', () => {
  afterEach(() => vi.restoreAllMocks());

  it('shows the rendered SVG as an image', async () => {
    const { fixture, renderer, settle, element } = await setUp();

    renderer.calls[0].finish(svg('A -> B'));
    await settle();

    expect(element.querySelector('img')?.getAttribute('src')).toBe('blob:image-1');
    expect(fixture.componentInstance.renderings).toEqual([svg('A -> B')]);
  });

  it('says why a source cannot be rendered', async () => {
    const { renderer, settle, element } = await setUp();

    renderer.calls[0].finish(
      new RockumlError('this diagram type is not ported yet', { notPorted: true, pageCount: 1 }),
    );
    await settle();

    expect(element.querySelector('img')).toBeNull();
    expect(element.textContent).toContain('this diagram type is not ported yet');
    expect(element.querySelector('rockuml-diagram')?.classList).toContain('rockuml-diagram--error');
  });

  it('keeps the newest source when an older rendering finishes last', async () => {
    const { fixture, renderer, settle, element } = await setUp();
    fixture.componentInstance.source.set('C -> D');
    await vi.waitFor(() => expect(renderer.calls.length).toBe(2));

    renderer.calls[1].finish(svg('C -> D'));
    await settle();
    renderer.calls[0].finish(svg('A -> B'));
    await settle();

    expect(fixture.componentInstance.renderings).toEqual([svg('C -> D')]);
    expect(element.querySelector('img')?.getAttribute('src')).toBe('blob:image-1');
  });
});
