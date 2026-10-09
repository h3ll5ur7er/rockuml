import {
  HighlightStyle,
  StreamLanguage,
  type StringStream,
  syntaxHighlighting,
} from '@codemirror/language';
import type { Extension } from '@codemirror/state';
import { tags } from '@lezer/highlight';

// Everything is defined on first use: module-level calls would keep CodeMirror in the bundles of applications
// that only show diagrams.

const KEYWORDS = `abstract activate actor agent alt annotation archimate artifact as autonumber binary box break card class clock
  cloud collections component concise control critical database deactivate destroy detach diamond else elseif
  end endif endwhile endfork endsplit endswitch entity enum exception file folder fork frame group hide if
  interface is json kill label left legend loop map metaclass namespace node note object of on opt over package
  par participant partition queue rectangle ref remove repeat return right robust skinparam split stack start
  state static stop storage struct switch then title together top bottom up down usecase while with`;

interface State {
  inBlockComment: boolean;
}

/** Highlighting for PlantUML sources: comments, `@start`/`@end` lines, preprocessor directives, keywords and arrows. */
function definePlantUml(): StreamLanguage<State> {
  const keywords = new Set(KEYWORDS.split(/\s+/));
  return StreamLanguage.define<State>({
    name: 'plantuml',
    startState: () => ({ inBlockComment: false }),
    token(stream, state) {
      if (state.inBlockComment) {
        state.inBlockComment = !skipPast(stream, "'/");
        return 'comment';
      }
      if (stream.eatSpace()) {
        return null;
      }
      const atLineStart = stream.string.slice(0, stream.pos).trim() === '';
      if (atLineStart && stream.match("'")) {
        stream.skipToEnd();
        return 'comment';
      }
      if (stream.match("/'")) {
        state.inBlockComment = !skipPast(stream, "'/");
        return 'comment';
      }
      if (atLineStart && stream.match(/^@(start|end)\w*/)) {
        return 'meta';
      }
      if (atLineStart && stream.match(/^!\w+/)) {
        return 'processingInstruction';
      }
      if (stream.match(/^"(?:[^"\\]|\\.)*"?/)) {
        return 'string';
      }
      if (stream.match(/^<<[^>]*>>/)) {
        return 'annotation';
      }
      if (stream.match(/^#(?:[0-9a-fA-F]{3,8}\b|[A-Za-z]+)/)) {
        return 'color';
      }
      if (stream.match(/^[$%][A-Za-z_]\w*/)) {
        return 'variableName';
      }
      if (stream.match(/^[-.=<>ox*|\\/[\]#+]*(?:->|<-|--|\.\.|==|-\[)[-.=<>ox*|\\/[\]#+]*/)) {
        return 'operator';
      }
      if (stream.match(/^\d+(?:\.\d+)?/)) {
        return 'number';
      }
      const word = stream.match(/^[A-Za-z_]\w*/);
      if (word) {
        return keywords.has((word as RegExpMatchArray)[0].toLowerCase()) ? 'keyword' : null;
      }
      stream.next();
      return null;
    },
    languageData: {
      commentTokens: { line: "'", block: { open: "/'", close: "'/" } },
    },
  });
}

/** Moves past `end` on this line; false if the line ends first. */
function skipPast(stream: StringStream, end: string): boolean {
  if (stream.skipTo(end)) {
    stream.pos += end.length;
    return true;
  }
  stream.skipToEnd();
  return false;
}

/** Colours taken from CSS custom properties, so that pages theme the editor along with themselves. */
function defineHighlightStyle(): HighlightStyle {
  return HighlightStyle.define([
    { tag: tags.comment, color: 'var(--rockuml-syntax-comment, #6b7280)', fontStyle: 'italic' },
    { tag: tags.meta, color: 'var(--rockuml-syntax-meta, #9333ea)', fontWeight: '600' },
    { tag: tags.processingInstruction, color: 'var(--rockuml-syntax-preprocessor, #c2410c)' },
    { tag: tags.keyword, color: 'var(--rockuml-syntax-keyword, #2563eb)' },
    { tag: tags.string, color: 'var(--rockuml-syntax-string, #15803d)' },
    { tag: tags.annotation, color: 'var(--rockuml-syntax-stereotype, #be185d)' },
    { tag: tags.color, color: 'var(--rockuml-syntax-color, #b45309)' },
    { tag: tags.variableName, color: 'var(--rockuml-syntax-variable, #0e7490)' },
    { tag: tags.operator, color: 'var(--rockuml-syntax-arrow, #dc2626)' },
    { tag: tags.number, color: 'var(--rockuml-syntax-number, #0f766e)' },
  ]);
}

let language: Extension | undefined;

/** The PlantUML language with its highlighting, for CodeMirror editors. */
export function plantUmlLanguage(): Extension {
  language ??= [definePlantUml(), syntaxHighlighting(defineHighlightStyle())];
  return language;
}
