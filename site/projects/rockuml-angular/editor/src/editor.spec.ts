import { Component, signal } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { EditorView } from '@codemirror/view';

import { RockumlEditor } from './editor';

@Component({
  imports: [RockumlEditor],
  template: `<rockuml-editor [(source)]="source" />`,
})
class Host {
  readonly source = signal('@startuml\nA -> B\n@enduml');
}

async function setUp() {
  const fixture = TestBed.createComponent(Host);
  await fixture.whenStable();
  const editor = (fixture.nativeElement as HTMLElement).querySelector('.cm-editor') as HTMLElement;
  return { fixture, view: EditorView.findFromDOM(editor)! };
}

describe('RockumlEditor', () => {
  it('shows the source', async () => {
    const { view } = await setUp();

    expect(view.state.doc.toString()).toBe('@startuml\nA -> B\n@enduml');
  });

  it('passes edits on to the source', async () => {
    const { fixture, view } = await setUp();

    view.dispatch({ changes: { from: 10, to: 16, insert: 'Marvin -> Arthur' } });

    expect(fixture.componentInstance.source()).toBe('@startuml\nMarvin -> Arthur\n@enduml');
  });

  it('takes sources set from outside', async () => {
    const { fixture, view } = await setUp();

    fixture.componentInstance.source.set('@startmindmap\n* 42\n@endmindmap');
    await fixture.whenStable();

    expect(view.state.doc.toString()).toBe('@startmindmap\n* 42\n@endmindmap');
  });
});
