import { TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';
import { EditorView } from '@codemirror/view';
import { ROCKUML_LOADER, type Rockuml } from '@rockuml/angular';

import { Example } from './example';

const SOURCE = '@startuml\nArthur -> Ford : towel?\n@enduml';

async function setUp() {
  const engine = { render: async () => ({ data: '<svg/>', pageCount: 1, isError: false }) };
  TestBed.configureTestingModule({
    providers: [
      provideRouter([]),
      { provide: ROCKUML_LOADER, useValue: async () => engine as unknown as Rockuml },
    ],
  });
  vi.spyOn(URL, 'createObjectURL').mockReturnValue('blob:diagram');
  const fixture = TestBed.createComponent(Example);
  fixture.componentRef.setInput('source', SOURCE);
  await fixture.whenStable();
  const element = fixture.nativeElement as HTMLElement;
  const view = EditorView.findFromDOM(element.querySelector('.cm-editor') as HTMLElement)!;
  const button = (label: string) =>
    [...element.querySelectorAll('button')].find((candidate) =>
      candidate.textContent?.includes(label),
    );
  return { fixture, view, button };
}

describe('Example', () => {
  afterEach(() => vi.restoreAllMocks());

  it('offers Reset only once the source is edited, and Reset restores it', async () => {
    const { fixture, view, button } = await setUp();
    expect(button('Reset')).toBeUndefined();

    view.dispatch({ changes: { from: 10, to: 16, insert: 'Zaphod' } });
    await fixture.whenStable();
    button('Reset')!.click();
    await fixture.whenStable();

    expect(view.state.doc.toString()).toBe(SOURCE);
    expect(button('Reset')).toBeUndefined();
  });

  it('copies the edited source', async () => {
    const writeText = vi.fn(async () => undefined);
    Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true });
    const { fixture, view, button } = await setUp();

    view.dispatch({ changes: { from: 10, to: 16, insert: 'Zaphod' } });
    button('Copy')!.click();
    await fixture.whenStable();

    expect(writeText).toHaveBeenCalledWith('@startuml\nZaphod -> Ford : towel?\n@enduml');
    expect(button('Copied!')).toBeDefined();
  });
});
