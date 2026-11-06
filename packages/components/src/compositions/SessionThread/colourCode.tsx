import { cssLanguage } from "@codemirror/lang-css";
import {
  javascriptLanguage,
  jsxLanguage,
  tsxLanguage,
  typescriptLanguage,
} from "@codemirror/lang-javascript";
import { jsonLanguage } from "@codemirror/lang-json";
import { pythonLanguage } from "@codemirror/lang-python";
import { rustLanguage } from "@codemirror/lang-rust";
import { yamlLanguage } from "@codemirror/lang-yaml";
import { StreamLanguage } from "@codemirror/language";
import { shell } from "@codemirror/legacy-modes/mode/shell";
import { toml } from "@codemirror/legacy-modes/mode/toml";
import { highlightCode, tagHighlighter, tags } from "@lezer/highlight";
import type { ReactNode } from "react";

// The file editor's grammars, by a fence's language name. A name not here stays plain.
type Parser = ReturnType<typeof StreamLanguage.define>["parser"];

const shellParser = StreamLanguage.define(shell).parser;
const PARSERS: Record<string, Parser> = {
  ts: typescriptLanguage.parser,
  typescript: typescriptLanguage.parser,
  tsx: tsxLanguage.parser,
  js: javascriptLanguage.parser,
  javascript: javascriptLanguage.parser,
  jsx: jsxLanguage.parser,
  json: jsonLanguage.parser,
  rust: rustLanguage.parser,
  rs: rustLanguage.parser,
  python: pythonLanguage.parser,
  py: pythonLanguage.parser,
  css: cssLanguage.parser,
  yaml: yamlLanguage.parser,
  yml: yamlLanguage.parser,
  toml: StreamLanguage.define(toml).parser,
  sh: shellParser,
  bash: shellParser,
  zsh: shellParser,
  shell: shellParser,
};

// Classes and not styles: the hues are the editor's roles, set by the stylesheet that draws the block.
const roles = tagHighlighter([
  {
    tag: [
      tags.keyword,
      tags.modifier,
      tags.operatorKeyword,
      tags.controlKeyword,
      tags.definitionKeyword,
    ],
    class: "tok-keyword",
  },
  {
    tag: [
      tags.string,
      tags.special(tags.string),
      tags.regexp,
    ],
    class: "tok-string",
  },
  {
    tag: [
      tags.number,
      tags.bool,
      tags.null,
      tags.atom,
      tags.literal,
    ],
    class: "tok-literal",
  },
  {
    tag: [
      tags.function(tags.variableName),
      tags.function(tags.propertyName),
      tags.propertyName,
      tags.typeName,
      tags.className,
      tags.namespace,
      tags.tagName,
    ],
    class: "tok-name",
  },
  {
    tag: tags.comment,
    class: "tok-comment",
  },
  {
    tag: [
      tags.punctuation,
      tags.bracket,
      tags.operator,
    ],
    class: "tok-punctuation",
  },
]);

/** A fenced block's text in spans classed by role, for `ProseCodeColour`; plain text where the language is not known. */
export function colourCode(language: string | undefined, source: string): ReactNode {
  const parser = language === undefined ? undefined : PARSERS[language.toLowerCase()];
  if (parser === undefined) return source;
  const out: ReactNode[] = [];
  highlightCode(
    source,
    parser.parse(source),
    roles,
    (text, classes) => {
      out.push(
        classes === "" ? (
          text
        ) : (
          <span key={out.length} className={classes}>
            {text}
          </span>
        ),
      );
    },
    () => {
      out.push("\n");
    },
  );
  return out;
}
