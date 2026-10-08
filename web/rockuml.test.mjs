// Acceptance tests of the wasm package against the golden model. After tools/build-wasm.sh:
//   node --test web/rockuml.test.mjs

import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { before, describe, test } from 'node:test';

import { load, RockumlError } from './rockuml.js';

const corpus = new URL('../tests/corpus/', import.meta.url);

let rockuml;
before(async () => {
  rockuml = await load(await readFile(new URL('rockuml.wasm', import.meta.url)));
});

async function source(id) {
  return readFile(new URL(`${id}.puml`, corpus), 'utf8');
}

// The golden of `page`: the CLI names later pages `<stem>_001` and so on.
async function golden(id, extension, page = 0) {
  const stem = id.split('/').pop();
  const suffix = page === 0 ? '' : `_${String(page).padStart(3, '0')}`;
  return normalise(await readFile(new URL(`${id}.golden/${stem}${suffix}.${extension}`, corpus), 'utf8'));
}

// As the parity test does: line endings depend on the checkout, and debug output stamps the render time.
function normalise(text) {
  return text
    .replaceAll('\r\n', '\n')
    .replace(/(Mon|Tue|Wed|Thu|Fri|Sat|Sun) [A-Z][a-z]{2} \d{2} \d{2}:\d{2}:\d{2} \S+ \d{4}/g, '<timestamp>');
}

async function rendered(id, format, page = 0) {
  return normalise((await rockuml.render(await source(id), { format, page })).data);
}

const CASES = [
  'sequence/activation',
  'class/hide-show',
  'activity/action-list',
  'state/basic',
  'salt/login',
  'usecase/actors-syntax',
];

describe('renders like PlantUML', () => {
  for (const id of CASES) {
    test(`${id} as svg`, async () => assert.equal(await rendered(id, 'svg'), await golden(id, 'svg')));
    test(`${id} as debug`, async () => assert.equal(await rendered(id, 'debug'), await golden(id, 'debug')));
  }

  test('deterministic svg', async () => {
    assert.equal(await rendered('state/basic', 'svg-deterministic'), await golden('state/basic', 'dsvg'));
  });

  test('preprocessed text', async () => {
    const id = 'preprocessor/builtins-color';
    assert.equal(await rendered(id, 'preproc'), await golden(id, 'preproc'));
  });

  test('png of the golden size', async () => {
    const id = 'salt/login';
    const { data } = await rockuml.render(await source(id), { format: 'png' });
    const expected = await readFile(new URL(`${id}.golden/login.png`, corpus));
    assert.deepEqual(pngSize(data), pngSize(expected));
  });

  test('every page', async () => {
    const id = 'sequence/newpage-autonumber';
    const { pageCount } = await rockuml.render(await source(id));
    assert.equal(pageCount, 2);
    for (let page = 0; page < pageCount; page++) {
      assert.equal(await rendered(id, 'svg', page), await golden(id, 'svg', page));
    }
  });
});

function pngSize(png) {
  const view = new DataView(png.buffer, png.byteOffset, png.byteLength);
  return [view.getUint32(16), view.getUint32(20)];
}

describe('reports what it cannot render', () => {
  test('diagrams with errors render as error images', async () => {
    const { data, isError } = await rockuml.render(await source('error/class-without-allowmixing'));
    assert.equal(isError, true);
    // Like PlantUML's, a source that is no file is called "string".
    assert.match(data, /\[From string \(line 3\) \]/);
    assert.match(data, /Use 'allowmixing' if you want to mix classes and other UML elements/);
  });

  test('the standard library is left out like a library that does not exist', async () => {
    const including = async (library) => {
      const { data, isError } = await rockuml.render(`@startuml\n!include <${library}>\nA -> B\n@enduml`, {
        format: 'debug',
      });
      // The seed comes from the source, so only the texts are compared.
      const texts = data.split('\n').filter((line) => line.includes('text:'));
      return { isError, texts: texts.join('\n').replace(library, '<library>') };
    };
    const missing = await including('nosuchlibrary/file');
    assert.equal(missing.isError, true);
    assert.match(missing.texts, /Fatal parsing error/);
    assert.deepEqual(await including('C4/C4_Container'), missing);
  });

  test('emoji are left out, so they show as unknown emoji do', async () => {
    const { data } = await rockuml.render('@startuml\nA -> B : <:smile:>\n@enduml', { format: 'debug' });
    assert.match(data, /text: ¿smile\?/);
  });

  test('diagram types that are not ported yet', async () => {
    await assert.rejects(rockuml.render('@startgantt\n[Task] lasts 2 days\n@endgantt'), {
      name: 'RockumlError',
      message: 'this diagram type is not ported yet',
      notPorted: true,
      pageCount: 1,
    });
  });

  test('sources without diagrams', async () => {
    await assert.rejects(rockuml.render('A -> B'), (error) => {
      assert.ok(error instanceof RockumlError);
      assert.equal(error.notPorted, false);
      assert.equal(error.pageCount, 0);
      return true;
    });
  });

  test('pages past the last', async () => {
    await assert.rejects(rockuml.render('@startuml\nA -> B\n@enduml', { page: 1 }), RockumlError);
  });

  test('unknown formats', async () => {
    await assert.rejects(rockuml.render('@startuml\nA -> B\n@enduml', { format: 'pdf' }), TypeError);
  });
});

describe('the host', () => {
  test('dates are in the local time zone', async (context) => {
    const timeZone = process.env.TZ;
    context.after(() => {
      if (timeZone === undefined) {
        delete process.env.TZ;
      } else {
        process.env.TZ = timeZone;
      }
    });
    process.env.TZ = 'Asia/Kolkata';
    const { data } = await rockuml.render('@startuml\nA -> B : %date("Z")\n@enduml', { format: 'preproc' });
    assert.match(data, /A -> B : \+0530/);
  });

  test('deeply nested diagrams fit on the stack', async () => {
    const depth = 800;
    const source = `@startuml\nstart\n${'if (c) then (yes)\n'.repeat(depth)}:x;\n${'endif\n'.repeat(depth)}@enduml`;
    const { isError } = await rockuml.render(source);
    assert.equal(isError, false);
  });

  test('rendering goes on after a stack overflow', async () => {
    const depth = 5000;
    const source = `@startuml\nstart\n${'if (c) then (yes)\n'.repeat(depth)}:x;\n${'endif\n'.repeat(depth)}@enduml`;
    await assert.rejects(rockuml.render(source), RangeError);
    const { isError } = await rockuml.render('@startuml\nA -> B\n@enduml');
    assert.equal(isError, false);
  });
});
