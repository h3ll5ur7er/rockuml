import {
  ChangeDetectionStrategy,
  Component,
  DestroyRef,
  ElementRef,
  afterNextRender,
  effect,
  inject,
  input,
  output,
  signal,
} from '@angular/core';
import { RockumlError, type Rendering } from '@rockuml/core';

import { RockumlRenderer } from './renderer';

/**
 * A diagram drawn from its source, as SVG. It renders once it scrolls into view, and again whenever the source
 * changes. The image is shown through an `<img>`, so scripts and links a source might put in the SVG stay inert.
 */
@Component({
  selector: 'rockuml-diagram',
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: {
    class: 'rockuml-diagram',
    '[class.rockuml-diagram--pending]': 'pending()',
    '[class.rockuml-diagram--error]': 'isError()',
    '[attr.aria-busy]': 'pending()',
  },
  template: `
    @if (imageUrl(); as url) {
      <img [src]="url" [alt]="alt()" />
    } @else if (message(); as text) {
      <p class="rockuml-diagram__message" role="status">{{ text }}</p>
    } @else {
      <div class="rockuml-diagram__placeholder" aria-hidden="true"></div>
    }
  `,
  styles: `
    :host {
      display: block;
    }
    img {
      display: block;
      max-width: var(--rockuml-diagram-max-width, 100%);
      max-height: var(--rockuml-diagram-max-height, none);
      height: auto;
      object-fit: contain;
      transition: opacity 120ms;
    }
    :host(.rockuml-diagram--pending) img {
      opacity: 0.6;
    }
    .rockuml-diagram__message {
      margin: 0;
      color: var(--rockuml-message-color, #b91c1c);
    }
    .rockuml-diagram__placeholder {
      min-width: 4rem;
      min-height: 4rem;
    }
  `,
})
export class RockumlDiagram {
  readonly source = input.required<string>();
  /** The page to show, counted over all diagrams of the source. */
  readonly page = input(0);
  readonly alt = input('Diagram');
  /** How long to wait after the source last changed before rendering, so that typing doesn't render every key. */
  readonly delay = input(0);

  /** Every SVG the diagram shows, error images included. */
  readonly rendered = output<Rendering<string>>();

  protected readonly imageUrl = signal<string | undefined>(undefined);
  protected readonly message = signal<string | undefined>(undefined);
  protected readonly pending = signal(false);
  protected readonly isError = signal(false);

  private readonly renderer = inject(RockumlRenderer);
  private readonly visible = signal(typeof IntersectionObserver === 'undefined');
  /** Numbers the renderings, so that a slow one finishing late can't replace a newer one. */
  private latest = 0;

  constructor() {
    const host = inject<ElementRef<HTMLElement>>(ElementRef).nativeElement;
    const destroyRef = inject(DestroyRef);
    afterNextRender(() => {
      if (this.visible()) {
        return;
      }
      const observer = new IntersectionObserver(
        (entries) => {
          if (entries.some((entry) => entry.isIntersecting)) {
            this.visible.set(true);
            observer.disconnect();
          }
        },
        { rootMargin: '400px' },
      );
      observer.observe(host);
      destroyRef.onDestroy(() => observer.disconnect());
    });

    effect((onCleanup) => {
      if (!this.visible()) {
        return;
      }
      const source = this.source();
      const page = this.page();
      const timer = setTimeout(() => this.draw(source, page), this.delay());
      onCleanup(() => clearTimeout(timer));
    });

    destroyRef.onDestroy(() => {
      this.latest++;
      this.showImage(undefined);
    });
  }

  private async draw(source: string, page: number): Promise<void> {
    const ticket = ++this.latest;
    this.pending.set(true);
    try {
      const rendering = await this.renderer.render(source, { page });
      if (ticket === this.latest) {
        this.showImage(rendering.data);
        this.isError.set(rendering.isError);
        this.rendered.emit(rendering);
      }
    } catch (error) {
      if (ticket === this.latest) {
        this.showImage(undefined);
        this.isError.set(true);
        this.message.set(describe(error));
      }
    } finally {
      if (ticket === this.latest) {
        this.pending.set(false);
      }
    }
  }

  private showImage(svg: string | undefined): void {
    const previous = this.imageUrl();
    if (previous) {
      URL.revokeObjectURL(previous);
    }
    this.imageUrl.set(
      svg === undefined
        ? undefined
        : URL.createObjectURL(new Blob([svg], { type: 'image/svg+xml' })),
    );
    this.message.set(undefined);
  }
}

function describe(error: unknown): string {
  if (error instanceof RockumlError) {
    return error.message;
  }
  return `rockuml could not render this diagram: ${error instanceof Error ? error.message : String(error)}`;
}
