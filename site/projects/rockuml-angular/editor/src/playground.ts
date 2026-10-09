import { ChangeDetectionStrategy, Component, input, model, output } from '@angular/core';
import type { Rendering } from 'rockuml';

import { RockumlDiagram } from 'rockuml-angular';
import { RockumlEditor } from './editor';

/** An editor with the diagram of its source next to it, redrawn as you type. */
@Component({
  selector: 'rockuml-playground',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [RockumlEditor, RockumlDiagram],
  host: { class: 'rockuml-playground' },
  template: `
    <div class="rockuml-playground__layout">
      <rockuml-editor class="rockuml-playground__editor" [(source)]="source" [label]="label()" />
      <div class="rockuml-playground__preview">
        <rockuml-diagram
          [source]="source()"
          [page]="page()"
          [alt]="alt()"
          [delay]="delay()"
          (rendered)="rendered.emit($event)"
        />
      </div>
    </div>
  `,
  styles: `
    :host {
      display: block;
      container-type: inline-size;
      min-height: 0;
    }
    .rockuml-playground__layout {
      display: grid;
      grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
      height: 100%;
      max-height: var(--rockuml-playground-max-height, none);
    }
    .rockuml-playground__editor {
      min-height: 0;
      overflow: auto;
      border-right: var(--rockuml-playground-divider, none);
    }
    .rockuml-playground__preview {
      display: grid;
      place-items: var(--rockuml-preview-align, safe center);
      min-height: 0;
      overflow: auto;
      padding: var(--rockuml-preview-padding, 16px);
      background: var(--rockuml-preview-background, #ffffff);
    }
    /* Stacked, the editor gets a height of its own, and the diagram the rest. */
    @container (max-width: 640px) {
      .rockuml-playground__layout {
        grid-template-columns: minmax(0, 1fr);
        grid-template-rows: auto minmax(0, 1fr);
        max-height: none;
      }
      .rockuml-playground__editor {
        max-height: var(--rockuml-stacked-editor-max-height, 50vh);
        border-right: none;
        border-bottom: var(--rockuml-playground-divider, none);
      }
    }
  `,
})
export class RockumlPlayground {
  readonly source = model.required<string>();
  readonly page = input(0);
  readonly label = input('Diagram source');
  readonly alt = input('Diagram');
  /** How long typing has to pause before the diagram is redrawn, in milliseconds. */
  readonly delay = input(150);

  readonly rendered = output<Rendering<string>>();
}
