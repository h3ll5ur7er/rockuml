import {
  ChangeDetectionStrategy,
  Component,
  afterRenderEffect,
  computed,
  inject,
  input,
  resource,
} from '@angular/core';
import { toSignal } from '@angular/core/rxjs-interop';
import { DomSanitizer, Title } from '@angular/platform-browser';
import { ActivatedRoute, Router, RouterLink } from '@angular/router';

import { Example } from '../shared/example';
import { parseMarkdown } from './markdown';
import { DOC_PAGES, findDocPage } from './pages';

@Component({
  selector: 'site-doc-page',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [RouterLink, Example],
  templateUrl: './doc-page.html',
  styleUrl: './doc-page.css',
})
export class DocPageView {
  /** From the route. */
  readonly slug = input.required<string>();

  protected readonly page = computed(() => findDocPage(this.slug()));
  protected readonly neighbours = computed(() => {
    const index = DOC_PAGES.findIndex((page) => page.slug === this.slug());
    return { previous: DOC_PAGES[index - 1], next: DOC_PAGES[index + 1] };
  });

  private readonly sanitizer = inject(DomSanitizer);
  private readonly router = inject(Router);
  private readonly route = inject(ActivatedRoute);
  private readonly fragment = toSignal(this.route.fragment);

  private readonly content = resource({
    params: () => this.page(),
    loader: async ({ params: page }) => parseMarkdown(await page.load()),
  });
  protected readonly headings = computed(() => this.content.value()?.headings ?? []);
  /** The pages' HTML is ours, bundled with the site, so it is trusted as it is. */
  protected readonly blocks = computed(() =>
    (this.content.value()?.blocks ?? []).map((block) =>
      block.kind === 'html'
        ? { ...block, trusted: this.sanitizer.bypassSecurityTrustHtml(block.html) }
        : block,
    ),
  );

  constructor() {
    const title = inject(Title);
    afterRenderEffect(() => {
      const page = this.page();
      title.setTitle(page ? `${page.title} · rockuml` : 'Not found · rockuml');
    });

    // The router scrolls to a fragment before the page's content has loaded, so the page does it once it has.
    afterRenderEffect(() => {
      const fragment = this.fragment();
      if (this.content.value() && fragment) {
        document.getElementById(fragment)?.scrollIntoView();
      }
    });
  }

  /** Links in the page's HTML go through the router instead of reloading the site. */
  protected followLink(event: MouseEvent): void {
    const link = (event.target as HTMLElement).closest('a');
    const href = link?.getAttribute('href');
    const modified = event.ctrlKey || event.metaKey || event.shiftKey || event.button !== 0;
    if (!href || link?.target || modified || /^[a-z]+:/i.test(href)) {
      return;
    }
    event.preventDefault();
    if (href.startsWith('#')) {
      void this.router.navigate([], { relativeTo: this.route, fragment: href.slice(1) });
    } else {
      void this.router.navigateByUrl(`/${href}`);
    }
  }
}
