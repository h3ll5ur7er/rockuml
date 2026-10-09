```rockuml tldr
@startuml
package "rockuml-angular" {
  component "provideRockuml()" as Provide
  component RockumlRenderer as Renderer
  component "<rockuml-diagram>" as Diagram
  component "<rockuml-editor>" as Editor
  component "<rockuml-playground>" as Playground
}
package rockuml {
  component "rockuml.js" as JS
  artifact "rockuml.wasm" as Wasm
}
Provide ..> Renderer : configures
Diagram --> Renderer
Playground *-- Editor
Playground *-- Diagram
Renderer --> JS
JS --> Wasm
@enduml
```

`rockuml-angular` is a library of Angular components for live diagrams: `<rockuml-diagram>` draws a source, `<rockuml-editor>` edits one with syntax highlighting, and `<rockuml-playground>` puts the two side by side and redraws as you type. They are standalone, signal-based components for Angular 22, and they power every example on this site.

The library has two entry points. `rockuml-angular` renders diagrams; `rockuml-angular/editor` adds the editors, built on [CodeMirror 6](https://codemirror.net). An application that only shows diagrams imports the first one and never downloads an editor.

> **Not on npm yet.** The library lives in `site/projects/rockuml-angular` in the repository and is built with `ng build rockuml-angular`. It is shaped to be published on its own, so it can become an npm package without changes.

## Setup

The library renders with the [JavaScript module](docs/javascript), the `rockuml` package. Serve `rockuml.wasm` with your application, for example by copying it among the assets in `angular.json`:

```json
{
  "glob": "rockuml.wasm",
  "input": "node_modules/rockuml",
  "output": "/"
}
```

Then tell the library where it is, once, in the application's providers:

```ts
import { ApplicationConfig } from '@angular/core';
import { provideRockuml } from 'rockuml-angular';

export const appConfig: ApplicationConfig = {
  providers: [provideRockuml({ wasm: 'rockuml.wasm' })],
};
```

The wasm module is downloaded the first time a diagram is drawn, not when the application starts.

## Diagrams

```ts
import { Component, signal } from '@angular/core';
import { RockumlDiagram } from 'rockuml-angular';

@Component({
  selector: 'app-architecture',
  imports: [RockumlDiagram],
  template: `<rockuml-diagram [source]="source()" alt="Our architecture" />`,
})
export class Architecture {
  readonly source = signal('@startuml\nFrontend -> Backend : REST\n@enduml');
}
```

| Input | Meaning |
|---|---|
| `source` | the diagram source (required) |
| `page` | the page to show, for sources with several pages |
| `alt` | the image's alternative text |
| `delay` | milliseconds to wait after a change before redrawing |

| Output | Meaning |
|---|---|
| `rendered` | every rendering, with the SVG text, the page count and whether it shows an error |

A diagram renders once it scrolls into view, so long pages full of diagrams stay fast. The SVG is shown through an `<img>`, which keeps any scripts or links a source might smuggle into it inert. A source that can't be rendered shows rockuml's message instead.

## The editor

`<rockuml-editor>`, from `rockuml-angular/editor`, binds two-way to a source and highlights comments, keywords, arrows, strings, colours, stereotypes and preprocessor lines:

```ts
import { Component, signal } from '@angular/core';
import { RockumlEditor } from 'rockuml-angular/editor';

@Component({
  selector: 'app-diagram-field',
  imports: [RockumlEditor],
  template: `<rockuml-editor [(source)]="source" label="Diagram source" />`,
})
export class DiagramField {
  readonly source = signal('@startuml
A -> B
@enduml');
}
```

The highlighting is also available for your own CodeMirror editors, as `plantUmlLanguage()`.

## The playground

`<rockuml-playground>`, also from `rockuml-angular/editor`, is an editor and a diagram side by side. It stacks them when it is narrower than 640 pixels.

```html
<rockuml-playground [(source)]="source" (rendered)="svg = $event.data" />
```

It takes the inputs of both, with a default `delay` of 150 milliseconds.

## Rendering from code

`RockumlRenderer` is the service behind the components. Inject it to render other formats, such as PNG for a download:

```ts
import { inject } from '@angular/core';
import { RockumlRenderer } from 'rockuml-angular';

export class DownloadButton {
  private readonly renderer = inject(RockumlRenderer);

  async download(source: string): Promise<Blob> {
    const { data } = await this.renderer.render(source, { format: 'png' });
    return new Blob([data], { type: 'image/png' });
  }
}
```

## Shareable links

`encodeSource(source)` turns a source into URL-safe text, and `decodeSource(text)` turns it back. The text is the source compressed in PlantUML's URL alphabet, so PlantUML servers can read it too. The [playground](playground) keeps its source in the address bar this way.

## Theming

The components take their colours from CSS custom properties, so they follow your application's theme:

| Property | Styles |
|---|---|
| `--rockuml-editor-background`, `--rockuml-editor-color` | the editor |
| `--rockuml-editor-font`, `--rockuml-editor-font-size` | the editor's font |
| `--rockuml-syntax-keyword`, `--rockuml-syntax-arrow`, `--rockuml-syntax-string`, `--rockuml-syntax-comment`, … | the highlighting |
| `--rockuml-preview-background`, `--rockuml-preview-padding` | the area around the diagram |
| `--rockuml-playground-divider` | the line between editor and diagram |
| `--rockuml-playground-max-height` | the playground's height limit |
| `--rockuml-diagram-max-width`, `--rockuml-diagram-max-height` | the diagram's size limits (`100%` and `none` unless set) |
| `--rockuml-message-color` | the colour of error messages |
