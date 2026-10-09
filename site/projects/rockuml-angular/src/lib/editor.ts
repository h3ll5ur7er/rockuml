import {
  ChangeDetectionStrategy,
  Component,
  DestroyRef,
  ElementRef,
  afterNextRender,
  effect,
  inject,
  input,
  model,
} from '@angular/core';
import { defaultKeymap, history, historyKeymap } from '@codemirror/commands';
import { EditorState } from '@codemirror/state';
import {
  EditorView,
  drawSelection,
  highlightActiveLine,
  keymap,
  lineNumbers,
} from '@codemirror/view';

import { plantUmlLanguage } from './language';

/** A CodeMirror editor for a diagram source, bound two-way to `source`. */
@Component({
  selector: 'rockuml-editor',
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: { class: 'rockuml-editor' },
  template: '',
  styles: `
    :host {
      display: block;
      min-height: 0;
    }
  `,
})
export class RockumlEditor {
  readonly source = model.required<string>();
  /** Names the editor for screen readers. */
  readonly label = input('Diagram source');

  private view?: EditorView;

  constructor() {
    const host = inject<ElementRef<HTMLElement>>(ElementRef).nativeElement;
    afterNextRender(() => {
      this.view = new EditorView({
        parent: host,
        state: EditorState.create({
          doc: this.source(),
          extensions: [
            lineNumbers(),
            history(),
            drawSelection(),
            highlightActiveLine(),
            EditorView.lineWrapping,
            keymap.of([...defaultKeymap, ...historyKeymap]),
            plantUmlLanguage(),
            theme,
            EditorView.contentAttributes.of({ 'aria-label': this.label() }),
            EditorView.updateListener.of((update) => {
              if (update.docChanged) {
                this.source.set(update.state.doc.toString());
              }
            }),
          ],
        }),
      });
    });

    // Sources set from outside (a reset, say) replace the text; the editor's own changes are already in it.
    effect(() => {
      const source = this.source();
      const view = this.view;
      if (view && source !== view.state.doc.toString()) {
        view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: source } });
      }
    });

    inject(DestroyRef).onDestroy(() => this.view?.destroy());
  }
}

const theme = EditorView.theme({
  '&': {
    height: '100%',
    fontSize: 'var(--rockuml-editor-font-size, 13px)',
    color: 'var(--rockuml-editor-color, inherit)',
    backgroundColor: 'var(--rockuml-editor-background, transparent)',
  },
  '&.cm-focused': { outline: 'none' },
  '.cm-scroller': {
    fontFamily:
      'var(--rockuml-editor-font, ui-monospace, SFMono-Regular, Menlo, Consolas, monospace)',
    lineHeight: '1.55',
    // Coding fonts would join arrows such as <-> into one sign, hiding what was typed.
    fontVariantLigatures: 'none',
  },
  '.cm-content': { caretColor: 'var(--rockuml-editor-caret, currentColor)' },
  '.cm-gutters': {
    color: 'var(--rockuml-editor-gutter-color, #9ca3af)',
    backgroundColor: 'var(--rockuml-editor-gutter-background, transparent)',
    border: 'none',
  },
  '.cm-activeLine, .cm-activeLineGutter': {
    backgroundColor: 'var(--rockuml-editor-active-line, rgba(127, 127, 127, 0.08))',
  },
  '&.cm-focused .cm-selectionBackground, .cm-selectionBackground': {
    backgroundColor: 'var(--rockuml-editor-selection, rgba(59, 130, 246, 0.25))',
  },
});
