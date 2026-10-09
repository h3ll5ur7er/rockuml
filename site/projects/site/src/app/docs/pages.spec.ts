import { DIAGRAM_PAGES, DOC_PAGES, findDocPage } from './pages';
import { loadTemplate } from './templates';

describe('the documentation pages', () => {
  it('have unique slugs', () => {
    const slugs = DOC_PAGES.map((page) => page.slug);

    expect(new Set(slugs).size).toBe(slugs.length);
  });

  it('each open with a TL;DR template', async () => {
    for (const page of DOC_PAGES) {
      expect(await loadTemplate(page), page.slug).toMatch(/^@start\w+/);
    }
  });

  it('list the diagram types for the gallery', () => {
    expect(DIAGRAM_PAGES.map((page) => page.slug)).toContain('sequence');
    expect(DIAGRAM_PAGES.every((page) => page.section === 'Diagrams')).toBe(true);
  });

  it('are found by slug', () => {
    expect(findDocPage('gantt')?.title).toBe('Gantt charts');
    expect(findDocPage('vogon-poetry')).toBeUndefined();
  });
});
