```rockuml tldr
@startuml
title Rendering in the browser
participant "Your page" as Page
participant "rockuml.js" as JS
participant "rockuml.wasm" as Wasm

Page -> JS : load()
JS -> Wasm : fetch and compile (once)
Page -> JS : render(source, { format: 'svg' })
JS -> Wasm : render
Wasm --> JS : SVG text
JS --> Page : { data, pageCount, isError }
@enduml
```

rockuml compiles to WebAssembly: `rockuml.wasm` is the whole engine, fonts and themes included, and `rockuml.js` is a small module without dependencies that loads it. Together they render diagrams in any modern browser and in Node, with no server and no other runtime. This site draws every one of its diagrams that way.

## Getting the files

Every release has a `rockuml-web.zip` with `rockuml.js`, `rockuml.wasm`, the TypeScript types and a demo page. To build them from source you need a Rust toolchain with the WebAssembly target:

```bash
rustup target add wasm32-unknown-unknown
bash tools/build-wasm.sh      # writes web/rockuml.wasm next to web/rockuml.js
```

The `web/` folder is laid out as an npm package called `rockuml`, with its `package.json` and `rockuml.d.ts`, so a project can depend on it by path until it is published:

```bash
npm install ../rockuml/web
```

## Rendering

```js
import { load, RockumlError } from './rockuml.js';

const rockuml = await load();   // fetches rockuml.wasm next to rockuml.js
const { data, pageCount, isError } = await rockuml.render(source, { format: 'svg', page: 0 });
document.querySelector('#diagram').innerHTML = data;
```

`load()` takes the wasm module's URL when it is served somewhere else, as it is after a bundler has moved `rockuml.js`: `load('/assets/rockuml.wasm')`. It also takes the module's bytes or a compiled `WebAssembly.Module`. Load once and render as often as you like; the module is about 10 MB, 5 MB compressed.

`render(source, options)` renders one page:

| Option | Values |
|---|---|
| `format` | `'svg'` (default), `'png'`, `'svg-deterministic'`, `'debug'` or `'preproc'` |
| `page` | the page to render, counted over all diagrams of the source; `0` by default |

It resolves to:

| Field | Meaning |
|---|---|
| `data` | the image: a string, or a `Uint8Array` for PNG |
| `pageCount` | the number of pages of all diagrams in the source |
| `isError` | the image shows a syntax error instead of the diagram |

## Errors

A diagram with a syntax error still renders: the image shows the error and the line it is on, and `isError` is true. `render` throws a `RockumlError` when there is nothing to render: a source without a diagram, a page past the last, or a diagram type rockuml has not ported yet. Its `notPorted` field tells the last case apart.

```js
try {
  const { data, isError } = await rockuml.render(source);
  show(data, isError);
} catch (error) {
  if (error instanceof RockumlError && error.notPorted) {
    showMessage('This diagram type is not supported yet.');
  } else {
    throw error;
  }
}
```

A source that nests deeper than the browser's stack allows throws a `RangeError`; rockuml recovers, and the next `render` works again.

## In Node

Node has no `fetch` for local files, so pass the bytes:

```js
import { readFile, writeFile } from 'node:fs/promises';
import { load } from 'rockuml';

const rockuml = await load(await readFile('node_modules/rockuml/rockuml.wasm'));
const { data } = await rockuml.render('@startuml\nA -> B\n@enduml', { format: 'png' });
await writeFile('diagram.png', data);
```

## TypeScript

`rockuml.d.ts` describes the module. `render` is typed by format: PNG gives a `Uint8Array`, every other format a string.

```ts
import { load, type Rendering } from 'rockuml';

const rockuml = await load();
const svg: Rendering<string> = await rockuml.render(source);
const png: Rendering<Uint8Array> = await rockuml.render(source, { format: 'png' });
```

## What the browser build leaves out

The browser has no files, so `!include` of files and URLs fails, and images can come only from data URIs. To keep the download small, the standard library (`!include <C4/…>`) and emoji are left out; they fail the way a library or emoji that doesn't exist does. Dates and `%date()` use the browser's time zone. Everything else, including every diagram type, themes, sprites and Open Iconic, works exactly as in the `rockuml` binary.

For Angular applications, the [Angular components](docs/angular) wrap all of this into a diagram tag and a live editor.
