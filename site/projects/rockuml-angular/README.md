# rockuml-angular

Angular components that draw PlantUML-compatible diagrams in the browser with
[rockuml](https://github.com/h3ll5ur7er/rockuml)'s WebAssembly engine: no server, no Java.

- `rockuml-angular`: `provideRockuml()`, `RockumlRenderer` and `<rockuml-diagram [source]="…">`.
- `rockuml-angular/editor`: `<rockuml-editor [(source)]="…">` and `<rockuml-playground [(source)]="…">`, built on
  CodeMirror 6, with PlantUML syntax highlighting.

Serve `rockuml.wasm` from the `rockuml` package among your assets and provide its URL:

```ts
providers: [provideRockuml({ wasm: 'rockuml.wasm' })];
```

The documentation is on rockuml's website, under "Angular components". Licensed under the LGPL 3.0 or later.
