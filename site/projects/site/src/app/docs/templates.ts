import { parseMarkdown } from './markdown';
import type { DocPage } from './pages';

/** The TL;DR template a page opens with. */
export async function loadTemplate(page: DocPage): Promise<string> {
  const { blocks } = parseMarkdown(await page.load());
  const template = blocks.find((block) => block.kind === 'example' && block.tldr);
  if (template?.kind !== 'example') {
    throw new Error(`the page ${page.slug} has no TL;DR template`);
  }
  return template.source;
}
