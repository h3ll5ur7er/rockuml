import { ChangeDetectionStrategy, Component, computed, signal } from '@angular/core';
import { RouterLink, RouterLinkActive, RouterOutlet } from '@angular/router';

import { DOC_SECTIONS } from './pages';

@Component({
  selector: 'site-docs-shell',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [RouterOutlet, RouterLink, RouterLinkActive],
  templateUrl: './docs-shell.html',
  styleUrl: './docs-shell.css',
})
export class DocsShell {
  protected readonly filter = signal('');
  protected readonly menuOpen = signal(false);

  protected readonly sections = computed(() => {
    const words = this.filter().toLowerCase().split(/\s+/).filter(Boolean);
    return DOC_SECTIONS.map((section) => ({
      title: section.title,
      pages: section.pages.filter((page) => {
        const text = `${page.title} ${page.summary}`.toLowerCase();
        return words.every((word) => text.includes(word));
      }),
    })).filter((section) => section.pages.length > 0);
  });
}
