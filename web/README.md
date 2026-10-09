# @rockuml/core

[rockuml](https://github.com/h3ll5ur7er/rockuml) renders PlantUML diagrams without Java or Graphviz. This package
is its engine compiled to WebAssembly, with a small JavaScript module without dependencies. It renders in
browsers and in Node, with no server.

```bash
npm install @rockuml/core
```

```js
import { load } from '@rockuml/core';

const rockuml = await load('/assets/rockuml.wasm'); // wherever you serve node_modules/@rockuml/core/rockuml.wasm
const { data, pageCount, isError } = await rockuml.render('@startuml\nAlice -> Bob : hi\n@enduml');
```

`render` makes `svg` (the default), `png`, `svg-deterministic`, `debug` or `preproc` output, for any page of the
source. In Node, pass the wasm module's bytes:
`load(await readFile('node_modules/@rockuml/core/rockuml.wasm'))`.

The documentation is at https://h3ll5ur7er.github.io/rockuml/docs/javascript, and every diagram there is
rendered by this package. For Angular, see [@rockuml/angular](https://www.npmjs.com/package/@rockuml/angular).

rockuml is a port of PlantUML by Arnaud Roques, distributed under the GNU Lesser General Public License 3.0
or later. NOTICE.md and THIRD-PARTY.md list the fonts, libraries and other works it carries.
