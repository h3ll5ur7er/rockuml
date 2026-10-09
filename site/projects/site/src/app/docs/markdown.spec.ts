import { parseMarkdown } from './markdown';

describe('parseMarkdown', () => {
  it('turns rockuml fences into examples and the rest into HTML', () => {
    const page = parseMarkdown(
      [
        '```rockuml tldr',
        '@startuml',
        'Arthur -> Ford : towel?',
        '@enduml',
        '```',
        '',
        'Some *text*.',
        '',
        '```rockuml',
        '@startmindmap',
        '* 42',
        '@endmindmap',
        '```',
      ].join('\n'),
    );

    expect(page.blocks).toEqual([
      { kind: 'example', source: '@startuml\nArthur -> Ford : towel?\n@enduml', tldr: true },
      { kind: 'html', html: '<p>Some <em>text</em>.</p>' },
      { kind: 'example', source: '@startmindmap\n* 42\n@endmindmap', tldr: false },
    ]);
  });

  it('writes other fences as escaped code', () => {
    const { blocks } = parseMarkdown('```bash\nrockuml --svg "a <b>.puml"\n```');

    expect(blocks).toEqual([
      {
        kind: 'html',
        html: '<pre class="code" data-language="bash"><code>rockuml --svg &quot;a &lt;b&gt;.puml&quot;</code></pre>',
      },
    ]);
  });

  it('gives headings unique ids and lists them', () => {
    const page = parseMarkdown('## Arrows & lines\n\n### Arrows & lines\n\n#### Deep');

    expect(page.blocks[0]).toEqual({
      kind: 'html',
      html:
        '<h2 id="arrows-lines"><a class="anchor" href="#arrows-lines" aria-hidden="true">#</a>Arrows &amp; lines</h2>' +
        '<h3 id="arrows-lines-2"><a class="anchor" href="#arrows-lines-2" aria-hidden="true">#</a>Arrows &amp; lines</h3>' +
        '<h4 id="deep"><a class="anchor" href="#deep" aria-hidden="true">#</a>Deep</h4>',
    });
    expect(page.headings).toEqual([
      { id: 'arrows-lines', text: 'Arrows & lines', level: 2 },
      { id: 'arrows-lines-2', text: 'Arrows & lines', level: 3 },
    ]);
  });

  it('formats inline code, emphasis and links', () => {
    const { blocks } = parseMarkdown(
      'Use `<b>` **boldly**, see [sequence](docs/sequence) or [PlantUML](https://plantuml.com).',
    );

    expect(blocks[0]).toEqual({
      kind: 'html',
      html:
        '<p>Use <code>&lt;b&gt;</code> <strong>boldly</strong>, see <a href="docs/sequence">sequence</a> or ' +
        '<a href="https://plantuml.com" target="_blank" rel="noopener">PlantUML</a>.</p>',
    });
  });

  it('keeps emphasis markers inside code as they are', () => {
    const { blocks } = parseMarkdown('Write `**` and `*` and `[x](y)`.');

    expect(blocks[0]).toEqual({
      kind: 'html',
      html: '<p>Write <code>**</code> and <code>*</code> and <code>[x](y)</code>.</p>',
    });
  });

  it('joins the lines of a paragraph', () => {
    const { blocks } = parseMarkdown('Don’t\npanic.\n\nAnd bring\na towel.');

    expect(blocks[0]).toEqual({
      kind: 'html',
      html: '<p>Don’t panic.</p><p>And bring a towel.</p>',
    });
  });

  it('nests lists by indentation', () => {
    const { blocks } = parseMarkdown('- one\n  more\n- two\n  - nested\n1. first\n2. second');

    expect(blocks[0]).toEqual({
      kind: 'html',
      html:
        '<ul><li>one more</li><li>two<ul><li>nested</li></ul></li></ul>' +
        '<ol><li>first</li><li>second</li></ol>',
    });
  });

  it('writes tables', () => {
    const { blocks } = parseMarkdown(
      '| Arrow | Means |\n|---|---|\n| `->` | call |\n| `-->` | reply |',
    );

    expect(blocks[0]).toEqual({
      kind: 'html',
      html:
        '<div class="table"><table><thead><tr><th>Arrow</th><th>Means</th></tr></thead><tbody>' +
        '<tr><td><code>-&gt;</code></td><td>call</td></tr>' +
        '<tr><td><code>--&gt;</code></td><td>reply</td></tr></tbody></table></div>',
    });
  });

  it('writes pipes inside table code as text', () => {
    const { blocks } = parseMarkdown('| Syntax |\n|---|\n| `a \\| b` |');

    expect(blocks[0]).toEqual({
      kind: 'html',
      html: '<div class="table"><table><thead><tr><th>Syntax</th></tr></thead><tbody><tr><td><code>a | b</code></td></tr></tbody></table></div>',
    });
  });

  it('makes quotes notes', () => {
    const { blocks } = parseMarkdown('> **Tip:** one\n> line.\n>\n> Another.');

    expect(blocks[0]).toEqual({
      kind: 'html',
      html: '<aside class="note"><p><strong>Tip:</strong> one line.</p><p>Another.</p></aside>',
    });
  });
});
