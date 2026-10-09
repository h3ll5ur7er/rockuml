// The Markdown of the documentation pages: the subset they use, with ```rockuml fences for live examples.

export type Block =
  | { kind: 'html'; html: string }
  /** A live example; the TL;DR template a page opens with is marked `tldr`. */
  | { kind: 'example'; source: string; tldr: boolean };

export interface Heading {
  id: string;
  text: string;
  level: number;
}

export interface DocContent {
  blocks: Block[];
  /** The page's second and third level headings, for its table of contents. */
  headings: Heading[];
}

const FENCE = /^```(\S*)\s*(.*)$/;
const HEADING = /^(#{2,4})\s+(.*)$/;
const LIST_ITEM = /^(\s*)([-*]|\d+\.)\s+(.*)$/;

export function parseMarkdown(markdown: string): DocContent {
  return new Parser(markdown.replace(/\r\n/g, '\n').split('\n')).parse();
}

class Parser {
  private index = 0;
  private html = '';
  private readonly blocks: Block[] = [];
  private readonly headings: Heading[] = [];
  private readonly ids = new Set<string>();

  constructor(private readonly lines: string[]) {}

  parse(): DocContent {
    while (this.index < this.lines.length) {
      const line = this.lines[this.index];
      if (line.trim() === '') {
        this.index++;
      } else if (FENCE.test(line)) {
        this.fence();
      } else if (HEADING.test(line)) {
        this.heading();
      } else if (line.startsWith('|')) {
        this.table();
      } else if (line.startsWith('>')) {
        this.note();
      } else if (LIST_ITEM.test(line)) {
        this.list();
      } else {
        this.html += `<p>${inline(this.paragraphText())}</p>`;
      }
    }
    this.flush();
    return { blocks: this.blocks, headings: this.headings };
  }

  private flush(): void {
    if (this.html) {
      this.blocks.push({ kind: 'html', html: this.html });
      this.html = '';
    }
  }

  private fence(): void {
    const [, language, info] = FENCE.exec(this.lines[this.index++])!;
    const body: string[] = [];
    while (this.index < this.lines.length && !this.lines[this.index].startsWith('```')) {
      body.push(this.lines[this.index++]);
    }
    this.index++;
    const text = body.join('\n');
    if (language === 'rockuml') {
      this.flush();
      this.blocks.push({ kind: 'example', source: text, tldr: info === 'tldr' });
    } else {
      const attribute = language ? ` data-language="${escape(language)}"` : '';
      this.html += `<pre class="code"${attribute}><code>${escape(text)}</code></pre>`;
    }
  }

  private heading(): void {
    const [, hashes, text] = HEADING.exec(this.lines[this.index++])!;
    const level = hashes.length;
    const id = this.uniqueId(slug(text));
    if (level <= 3) {
      this.headings.push({ id, text: plain(text), level });
    }
    this.html +=
      `<h${level} id="${id}"><a class="anchor" href="#${id}" aria-hidden="true">#</a>` +
      `${inline(text)}</h${level}>`;
  }

  private uniqueId(base: string): string {
    let id = base;
    for (let suffix = 2; this.ids.has(id); suffix++) {
      id = `${base}-${suffix}`;
    }
    this.ids.add(id);
    return id;
  }

  private table(): void {
    const rows: string[][] = [];
    while (this.index < this.lines.length && this.lines[this.index].startsWith('|')) {
      rows.push(cells(this.lines[this.index++]));
    }
    const [header, , ...body] = rows;
    const row = (values: string[], tag: string) =>
      `<tr>${values.map((value) => `<${tag}>${inline(value)}</${tag}>`).join('')}</tr>`;
    this.html +=
      `<div class="table"><table><thead>${row(header, 'th')}</thead>` +
      `<tbody>${body.map((values) => row(values, 'td')).join('')}</tbody></table></div>`;
  }

  private note(): void {
    const lines: string[] = [];
    while (this.index < this.lines.length && this.lines[this.index].startsWith('>')) {
      lines.push(this.lines[this.index++].replace(/^> ?/, ''));
    }
    const paragraphs = lines
      .join('\n')
      .split(/\n\s*\n/)
      .map((paragraph) => `<p>${inline(paragraph.replace(/\n/g, ' '))}</p>`);
    this.html += `<aside class="note">${paragraphs.join('')}</aside>`;
  }

  private list(): void {
    const items: { indent: number; ordered: boolean; text: string }[] = [];
    while (this.index < this.lines.length && this.lines[this.index].trim() !== '') {
      const line = this.lines[this.index++];
      const item = LIST_ITEM.exec(line);
      if (item) {
        items.push({ indent: item[1].length, ordered: /\d/.test(item[2]), text: item[3] });
      } else {
        items[items.length - 1].text += ` ${line.trim()}`;
      }
    }
    this.html += renderList(items, 0, items.length);
  }

  private paragraphText(): string {
    const lines: string[] = [];
    while (this.index < this.lines.length) {
      const line = this.lines[this.index];
      if (line.trim() === '' || FENCE.test(line) || HEADING.test(line) || /^[|>]/.test(line)) {
        break;
      }
      if (lines.length > 0 && LIST_ITEM.test(line)) {
        break;
      }
      lines.push(line.trim());
      this.index++;
    }
    return lines.join(' ');
  }
}

/** The items from `start` to `end` as lists: consecutive items of one indentation, more indented ones inside. */
function renderList(
  items: { indent: number; ordered: boolean; text: string }[],
  start: number,
  end: number,
): string {
  let html = '';
  let index = start;
  while (index < end) {
    const { indent, ordered } = items[index];
    const tag = ordered ? 'ol' : 'ul';
    html += `<${tag}>`;
    while (index < end && items[index].indent === indent && items[index].ordered === ordered) {
      let next = index + 1;
      while (next < end && items[next].indent > indent) {
        next++;
      }
      html += `<li>${inline(items[index].text)}${renderList(items, index + 1, next)}</li>`;
      index = next;
    }
    html += `</${tag}>`;
  }
  return html;
}

/** A table row's cells; `\|` is a pipe inside a cell. */
function cells(line: string): string[] {
  return line
    .trim()
    .replace(/^\||\|$/g, '')
    .split(/(?<!\\)\|/)
    .map((cell) => cell.trim().replace(/\\\|/g, '|'));
}

function inline(text: string): string {
  const codes: string[] = [];
  const protectedText = text.replace(
    /``(.+?)``|`([^`]+)`/g,
    (_, double: string, single: string) => {
      codes.push(escape(double ?? single));
      return `\u0000${codes.length - 1}\u0000`;
    },
  );
  return escape(protectedText)
    .replace(/\[([^\]]+)\]\(([^)\s]+)\)/g, (_, label: string, href: string) =>
      /^https?:/.test(href)
        ? `<a href="${href}" target="_blank" rel="noopener">${label}</a>`
        : `<a href="${href}">${label}</a>`,
    )
    .replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')
    .replace(/(^|[^*\w])\*(?!\s)(.+?)(?<!\s)\*(?!\w)/g, '$1<em>$2</em>')
    .replace(/\u0000(\d+)\u0000/g, (_, index: string) => `<code>${codes[Number(index)]}</code>`);
}

/** A heading's text without its Markdown, for the table of contents. */
function plain(text: string): string {
  return text.replace(/`|\*\*?/g, '');
}

function slug(text: string): string {
  return (
    plain(text)
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, '-')
      .replace(/^-|-$/g, '') || 'section'
  );
}

function escape(text: string): string {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}
