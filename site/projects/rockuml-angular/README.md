# @rockuml/angular

Angular components that draw PlantUML diagrams in the browser with
[rockuml](https://github.com/h3ll5ur7er/rockuml)'s WebAssembly engine, without a server or Java.

```bash
npm install @rockuml/angular @rockuml/core
```

- `@rockuml/angular`: `provideRockuml()`, `RockumlRenderer` and `<rockuml-diagram [source]="…">`.
- `@rockuml/angular/editor`: `<rockuml-editor [(source)]="…">` and `<rockuml-playground [(source)]="…">`,
  built on CodeMirror 6 with PlantUML syntax highlighting. It needs the `@codemirror/*` and `@lezer/highlight`
  packages; applications that only show diagrams can leave them out.

Serve `rockuml.wasm` from `node_modules/@rockuml/core` among your assets, and provide its URL:

```ts
providers: [provideRockuml({ wasm: 'rockuml.wasm' })];
```

The documentation is at https://h3ll5ur7er.github.io/rockuml/docs/angular. Licensed under the GNU Lesser General
Public License 3.0 or later.
