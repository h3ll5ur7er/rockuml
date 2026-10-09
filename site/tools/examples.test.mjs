// Renders every live example of the documentation with rockuml.wasm, so that no page shows a broken diagram.
//   node --test tools/examples.test.mjs

import assert from 'node:assert/strict';
import { readFile, readdir } from 'node:fs/promises';
import { before, describe, test } from 'node:test';

import { load } from 'rockuml';

const content = new URL('../projects/site/src/app/docs/content/', import.meta.url);
const wasm = new URL('../node_modules/rockuml/rockuml.wasm', import.meta.url);

const pages = (await readdir(content)).filter((name) => name.endsWith('.md')).sort();
const slugs = new Set(pages.map((name) => name.replace(/\.md$/, '')));

let rockuml;
before(async () => {
  rockuml = await load(await readFile(wasm));
});

function examples(markdown) {
  return [
    ...markdown.replace(/\r\n/g, '\n').matchAll(/^```rockuml( tldr)?\n([\s\S]*?)^```$/gm),
  ].map((match) => ({ tldr: Boolean(match[1]), source: match[2].replace(/\n$/, '') }));
}

/** The text of an error image, which says what went wrong. */
function texts(svg) {
  return [...svg.matchAll(/<text[^>]*>([^<]*)<\/text>/g)].map((match) => match[1]).join(' | ');
}

for (const name of pages) {
  const markdown = await readFile(new URL(name, content), 'utf8');
  const found = examples(markdown);

  describe(name, () => {
    test('opens with its one TL;DR template', () => {
      assert.ok(found.length > 0, 'the page has no example');
      assert.equal(found[0].tldr, true, 'the first example is not the TL;DR template');
      assert.equal(found.filter((example) => example.tldr).length, 1);
    });

    test('links to pages that exist', () => {
      for (const [, slug] of markdown.matchAll(/\]\(docs\/([\w-]+)/g)) {
        assert.ok(slugs.has(slug), `no page ${slug}`);
      }
    });

    found.forEach((example, index) => {
      const firstLine = example.source.split('\n').find((line) => !line.startsWith('@start')) ?? '';
      test(`example ${index + 1} renders: ${firstLine.slice(0, 50)}`, async () => {
        const first = await rockuml.render(example.source);
        for (let page = 0; page < first.pageCount; page++) {
          const { data, isError } =
            page === 0 ? first : await rockuml.render(example.source, { page });
          assert.equal(isError, false, `page ${page + 1} is an error image: ${texts(data)}`);
          assert.doesNotMatch(
            texts(data),
            /deprecated/i,
            `page ${page + 1} warns of deprecated syntax`,
          );
        }
      });
    });
  });
}
