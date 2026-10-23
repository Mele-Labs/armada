import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { css } from "@codemirror/lang-css";
import { html } from "@codemirror/lang-html";
import { javascript } from "@codemirror/lang-javascript";
import { json } from "@codemirror/lang-json";
import { markdown } from "@codemirror/lang-markdown";
import { python } from "@codemirror/lang-python";
import { rust } from "@codemirror/lang-rust";
import { yaml } from "@codemirror/lang-yaml";
import { HighlightStyle, StreamLanguage, bracketMatching, indentOnInput, syntaxHighlighting } from "@codemirror/language";
import type { Extension } from "@codemirror/state";
import { Compartment, EditorState, RangeSetBuilder } from "@codemirror/state";
import { shell } from "@codemirror/legacy-modes/mode/shell";
import { toml } from "@codemirror/legacy-modes/mode/toml";
import { highlightSelectionMatches, search, searchKeymap, searchPanelOpen } from "@codemirror/search";
import { Decoration, EditorView, GutterMarker, drawSelection, gutter, highlightActiveLine, highlightActiveLineGutter, keymap, lineNumbers } from "@codemirror/view";
import { tags } from "@lezer/highlight";
import { useEffect, useRef } from "react";

/**
 * A file's text in CodeMirror 6, drawn from the tokens so it follows dark and light with the rest of the window.
 *
 * **The editor lives in a shadow root.** CodeMirror writes its styles into a `<style>` element, which Bridge's CSP
 * (`default-src 'self'`) refuses; given a shadow root it adopts them with `adoptedStyleSheets` instead, and the policy
 * stays as it is. Two things follow from that. A press made inside reaches the window with the host as its target, so a
 * plain key is stopped at the host and Bridge's single-key shortcuts never hear it; a chord passes, for ⌘S and ⌘O.
 * And a walk finds the host, a group named for the file, not the text inside it (`typeIntoEditor`).
 */

/** What a name, by extension, is read as. */
export type CodeLanguage = "json" | "markdown" | "javascript" | "typescript" | "jsx" | "tsx" | "rust" | "yaml" | "css" | "html" | "python" | "toml" | "shell" | "text";

const BY_EXTENSION: Record<string, CodeLanguage> = {
  json: "json",
  jsonc: "json",
  md: "markdown",
  markdown: "markdown",
  js: "javascript",
  mjs: "javascript",
  cjs: "javascript",
  jsx: "jsx",
  ts: "typescript",
  mts: "typescript",
  cts: "typescript",
  tsx: "tsx",
  rs: "rust",
  yaml: "yaml",
  yml: "yaml",
  css: "css",
  html: "html",
  htm: "html",
  py: "python",
  toml: "toml",
  sh: "shell",
  bash: "shell",
  zsh: "shell",
};

/** The language a file name asks for, by its extension. A name with none is plain text. */
export function languageOf(name: string): CodeLanguage {
  const dot = name.lastIndexOf(".");
  return dot < 0 ? "text" : (BY_EXTENSION[name.slice(dot + 1).toLowerCase()] ?? "text");
}

const support = (language: CodeLanguage): Extension => {
  switch (language) {
    case "json":
      return json();
    case "markdown":
      return markdown();
    case "javascript":
      return javascript();
    case "typescript":
      return javascript({ typescript: true });
    case "jsx":
      return javascript({ jsx: true });
    case "tsx":
      return javascript({ typescript: true, jsx: true });
    case "rust":
      return rust();
    case "yaml":
      return yaml();
    case "css":
      return css();
    case "html":
      return html();
    case "python":
      return python();
    case "toml":
      return StreamLanguage.define(toml);
    case "shell":
      return StreamLanguage.define(shell);
    case "text":
      return [];
  }
};

// **No token is named for syntax yet**, so the roles borrow the three tool hues and the neutral steps, which have light
// and dark values. A `--syntax-*` set would replace this table and nothing else.
const highlight = HighlightStyle.define([
  { tag: [tags.keyword, tags.modifier, tags.operatorKeyword, tags.controlKeyword, tags.definitionKeyword], color: "var(--tool-change)" },
  { tag: [tags.string, tags.special(tags.string), tags.regexp], color: "var(--tool-run)" },
  { tag: [tags.number, tags.bool, tags.null, tags.atom, tags.literal], color: "var(--tool-run)" },
  { tag: [tags.function(tags.variableName), tags.function(tags.propertyName), tags.propertyName, tags.attributeName], color: "var(--tool-look)" },
  { tag: [tags.typeName, tags.className, tags.namespace, tags.tagName, tags.macroName], color: "var(--tool-look)" },
  { tag: [tags.comment, tags.docComment, tags.lineComment, tags.blockComment], color: "var(--fg-subtle)" },
  { tag: [tags.punctuation, tags.operator, tags.bracket, tags.separator, tags.meta], color: "var(--fg-muted)" },
  { tag: [tags.heading, tags.strong], color: "var(--fg-default)", fontWeight: "var(--weight-heading)" },
  { tag: [tags.link, tags.url], color: "var(--accent)" },
  { tag: tags.emphasis, fontStyle: "italic" },
  { tag: tags.strikethrough, textDecoration: "line-through" },
]);

const theme = EditorView.theme({
  "&": {
    height: "100%",
    backgroundColor: "var(--bg-sunken)",
    color: "var(--fg-default)",
    fontSize: "var(--text-xs)",
  },
  "&.cm-focused": { outline: "none" },
  ".cm-scroller": {
    fontFamily: "var(--font-mono)",
    lineHeight: "var(--leading-xs)",
    overflow: "auto",
  },
  ".cm-content": { caretColor: "var(--accent)", padding: "var(--space-2) 0" },
  ".cm-line": { padding: "0 var(--space-3)" },
  ".cm-cursor, .cm-dropCursor": { borderLeftColor: "var(--accent)" },
  ".cm-selectionBackground, &.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground, .cm-content ::selection": {
    backgroundColor: "var(--accent-muted)",
  },
  ".cm-activeLine": { backgroundColor: "var(--bg-hover)" },
  ".cm-gutters": {
    backgroundColor: "var(--bg-sunken)",
    color: "var(--fg-subtle)",
    border: "none",
  },
  ".cm-lineNumbers .cm-gutterElement": { padding: "0 var(--space-2) 0 var(--space-3)", minWidth: "var(--space-8)" },
  ".cm-activeLineGutter": { backgroundColor: "var(--bg-hover)", color: "var(--fg-muted)" },
  ".cm-matchingBracket, .cm-nonmatchingBracket": { backgroundColor: "var(--bg-hover)", outline: "var(--border-width) solid var(--border-strong)" },
  ".cm-selectionMatch": { backgroundColor: "var(--accent-faint)" },
  ".cm-searchMatch": { backgroundColor: "var(--accent-faint)", outline: "var(--border-width) solid var(--accent)" },
  ".cm-searchMatch.cm-searchMatch-selected": { backgroundColor: "var(--accent-muted)" },
  ".cm-panels": { backgroundColor: "var(--bg-raised)", color: "var(--fg-default)", fontFamily: "var(--font-sans)" },
  ".cm-panels.cm-panels-top": { borderBottom: "var(--border-width) solid var(--border-default)" },
  ".cm-panel.cm-search": { padding: "var(--space-2) var(--space-3)", fontSize: "var(--text-xs)" },
  ".cm-panel.cm-search input, .cm-panel.cm-search button, .cm-panel.cm-search label": { fontSize: "var(--text-xs)" },
  ".cm-textfield": {
    backgroundColor: "var(--bg-sunken)",
    color: "var(--fg-default)",
    border: "var(--border-width) solid var(--border-default)",
    borderRadius: "var(--radius-md)",
    padding: "var(--space-1) var(--space-2)",
  },
  ".cm-textfield:focus": { outline: "var(--focus-ring)", outlineOffset: "var(--focus-ring-offset)" },
  ".cm-button": {
    backgroundColor: "var(--bg-sunken)",
    backgroundImage: "none",
    color: "var(--fg-default)",
    border: "var(--border-width) solid var(--border-default)",
    borderRadius: "var(--radius-md)",
    padding: "var(--space-1) var(--space-2)",
  },
  ".cm-button:hover": { backgroundColor: "var(--bg-hover)" },
  ".cm-button:active": { backgroundColor: "var(--bg-hover)", backgroundImage: "none" },
  ".cm-panel.cm-search [name=close]": { color: "var(--fg-muted)" },
  // A held note's lines, and its number in a gutter of its own beside the line numbers. As `NoteNumber` draws one.
  ".cm-noted": { backgroundColor: "var(--accent-faint)", boxShadow: "inset var(--edge-active) 0 0 var(--accent)" },
  ".cm-notes-gutter .cm-gutterElement": { display: "flex", gap: "var(--space-1)", alignItems: "center", padding: "0 var(--space-1)" },
  ".cm-note-number": {
    display: "inline-flex",
    alignItems: "center",
    justifyContent: "center",
    boxSizing: "border-box",
    minWidth: "var(--leading-2xs)",
    height: "var(--leading-2xs)",
    padding: "0 var(--space-1)",
    border: "var(--border-width) solid var(--accent)",
    borderRadius: "var(--radius-sm)",
    backgroundColor: "var(--accent)",
    color: "var(--fg-inverse)",
    fontSize: "var(--text-2xs)",
    lineHeight: "1",
  },
  ".cm-note-number[data-writing]": { backgroundColor: "var(--bg-sunken)", color: "var(--accent)" },
});

/** A note held on the text, as the editor marks it: its number, and the lines it is about. */
export type CodeNoteMark = { n: number; from: number; to: number; writing: boolean };

/** Lines a press points at: the selection's whole lines, or the line under the pointer where nothing is selected. */
export type CodeLines = { from: number; to: number; quote: string };

class NoteNumbers extends GutterMarker {
  constructor(readonly marks: readonly CodeNoteMark[]) {
    super();
  }
  override eq(other: NoteNumbers): boolean {
    return other.marks.map((one) => `${String(one.n)}${String(one.writing)}`).join() === this.marks.map((one) => `${String(one.n)}${String(one.writing)}`).join();
  }
  override toDOM(): Node {
    const all = document.createElement("span");
    all.style.display = "contents";
    for (const one of this.marks) {
      const mark = document.createElement("span");
      mark.className = "cm-note-number";
      if (one.writing) mark.dataset.writing = "";
      mark.setAttribute("role", "img");
      mark.setAttribute("aria-label", `Note ${String(one.n)}`);
      mark.textContent = String(one.n);
      all.append(mark);
    }
    return all;
  }
}

/** The held notes' lines tinted, and each note's number at its first line. Nothing at all without notes, so no empty gutter stands. */
function noteMarks(marks: readonly CodeNoteMark[]): Extension {
  if (marks.length === 0) return [];
  const within = (view: EditorView, line: number) => Math.min(view.state.doc.lines, Math.max(1, line));
  return [
    EditorView.decorations.of((view) => {
      const lines = new Set<number>();
      for (const one of marks) for (let line = within(view, one.from); line <= within(view, one.to); line += 1) lines.add(line);
      const built = new RangeSetBuilder<Decoration>();
      for (const line of [...lines].sort((a, b) => a - b)) {
        const at = view.state.doc.line(line).from;
        built.add(at, at, Decoration.line({ class: "cm-noted" }));
      }
      return built.finish();
    }),
    gutter({
      class: "cm-notes-gutter",
      markers: (view) => {
        const byLine = new Map<number, CodeNoteMark[]>();
        for (const one of marks) byLine.set(within(view, one.from), [...(byLine.get(within(view, one.from)) ?? []), one]);
        const built = new RangeSetBuilder<GutterMarker>();
        for (const [line, here] of [...byLine].sort((a, b) => a[0] - b[0])) built.add(view.state.doc.line(line).from, view.state.doc.line(line).from, new NoteNumbers(here));
        return built.finish();
      },
    }),
  ];
}

/** Where `lines` are drawn: across the text, from the top of the first to the bottom of the last. Undefined off screen. */
function drawnOf(view: EditorView, lines: CodeLines): { left: number; top: number; right: number; bottom: number } | undefined {
  const doc = view.state.doc;
  const first = view.coordsAtPos(doc.line(lines.from).from);
  const last = view.coordsAtPos(doc.line(lines.to).to);
  const text = view.contentDOM.getBoundingClientRect();
  return first === null || last === null ? undefined : { left: text.left, top: first.top, right: text.right, bottom: last.bottom };
}

/** The whole lines from `from` to `to`, where a selection that ends at the start of a line does not take that line. */
function linesOf(view: EditorView, from: number, to: number): CodeLines {
  const doc = view.state.doc;
  const first = doc.lineAt(from);
  let last = doc.lineAt(to);
  if (to > from && to === last.from && last.number > first.number) last = doc.line(last.number - 1);
  return { from: first.number, to: last.number, quote: doc.sliceString(first.from, last.to) };
}

export type CodeEditorProps = {
  /** Names the editor, and picks the language by its extension. */
  name: string;
  value: string;
  /** Absent leaves the text read-only. */
  onChange?: (text: string) => void;
  language?: CodeLanguage;
  /** Moves focus to the text when it mounts. */
  autoFocus?: boolean;
  /** The line to bring to the top, as a fraction where it is part way down one. Moves the text, and does not report it. */
  scrollTo?: number;
  /** The line at the top, whenever a person scrolls the text. Not called for a move `scrollTo` made. */
  onScroll?: (line: number) => void;
  /** Held notes, marked on their lines. */
  notes?: readonly CodeNoteMark[];
  /** A right press, or the menu key or Shift F10: where, the lines it points at, and where they are on screen. */
  onNote?: (at: { x: number; y: number }, lines: CodeLines, drawn: { left: number; top: number; right: number; bottom: number } | undefined) => void;
};

/** The line at the top of a view scrolled to `top`, as a fraction where it is part way down one. */
function lineAt(view: EditorView, top: number): number {
  const block = view.lineBlockAtHeight(Math.max(0, top));
  const share = block.height > 0 ? Math.min(1, Math.max(0, (top - block.top) / block.height)) : 0;
  return view.state.doc.lineAt(block.from).number + share;
}

/** How far down a view a line is, the other way. */
function topOf(view: EditorView, line: number): number {
  const doc = view.state.doc;
  const whole = Math.min(doc.lines, Math.max(1, Math.floor(line)));
  const block = view.lineBlockAt(doc.line(whole).from);
  return block.top + Math.min(1, Math.max(0, line - whole)) * block.height;
}

export function CodeEditor({ name, value, onChange, language, autoFocus = false, scrollTo, onScroll, notes, onNote }: CodeEditorProps) {
  const host = useRef<HTMLDivElement>(null);
  const view = useRef<EditorView | null>(null);
  const change = useRef(onChange);
  change.current = onChange;
  const scrolled = useRef(onScroll);
  scrolled.current = onScroll;
  const noting = useRef(onNote);
  noting.current = onNote;
  // Where `scrollTo` last put the text: the scroll that move makes is its own, and is not a person's.
  const put = useRef<number | null>(null);
  const editable = onChange !== undefined;
  const kind = language ?? languageOf(name);
  const wraps = kind === "markdown" || kind === "text";
  const first = useRef({ value, kind, editable, wraps, name });
  const slots = useRef({ language: new Compartment(), editable: new Compartment(), wrap: new Compartment(), name: new Compartment(), notes: new Compartment() });

  useEffect(() => {
    const el = host.current;
    if (el === null) return;
    const root = el.shadowRoot ?? el.attachShadow({ mode: "open" });
    const start = first.current;
    const now = slots.current;
    const next = new EditorView({
      parent: root,
      state: EditorState.create({
        doc: start.value,
        extensions: [
          lineNumbers(),
          now.notes.of(noteMarks(notes ?? [])),
          highlightActiveLineGutter(),
          highlightActiveLine(),
          drawSelection(),
          history(),
          indentOnInput(),
          bracketMatching(),
          highlightSelectionMatches(),
          search({ top: true }),
          syntaxHighlighting(highlight),
          theme,
          keymap.of([...searchKeymap, ...historyKeymap, ...defaultKeymap]),
          now.language.of(support(start.kind)),
          now.wrap.of(start.wraps ? EditorView.lineWrapping : []),
          now.editable.of([EditorState.readOnly.of(!start.editable), EditorView.editable.of(start.editable)]),
          now.name.of(EditorView.contentAttributes.of({ "aria-label": start.name })),
          EditorView.updateListener.of((update) => {
            if (update.docChanged && update.transactions.some((one) => one.docChanged)) change.current?.(update.state.doc.toString());
            el.toggleAttribute("data-keeps-esc", searchPanelOpen(update.state));
          }),
        ],
      }),
    });
    view.current = next;
    // The scroller, not the content: a scroll does not reach the content's handlers.
    const scroller = next.scrollDOM;
    const scrolling = () => {
      const top = scroller.scrollTop;
      if (put.current !== null && Math.abs(top - put.current) < 2) {
        put.current = null;
        return;
      }
      put.current = null;
      scrolled.current?.(lineAt(next, top));
    };
    scroller.addEventListener("scroll", scrolling);
    if (autoFocus) next.focus();
    return () => {
      scroller.removeEventListener("scroll", scrolling);
      next.destroy();
      view.current = null;
    };
    // Mounted once: what changes afterwards is reconfigured below, so the history survives a save.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    const at = view.current;
    if (at === null || scrollTo === undefined) return;
    const top = topOf(at, scrollTo);
    if (Math.abs(top - at.scrollDOM.scrollTop) < 1) return;
    put.current = top;
    at.scrollDOM.scrollTop = top;
  }, [scrollTo]);

  useEffect(() => {
    const at = view.current;
    if (at === null || at.state.doc.toString() === value) return;
    at.dispatch({ changes: { from: 0, to: at.state.doc.length, insert: value } });
  }, [value]);

  useEffect(() => {
    const at = view.current;
    const now = slots.current;
    at?.dispatch({
      effects: [
        now.language.reconfigure(support(kind)),
        now.wrap.reconfigure(wraps ? EditorView.lineWrapping : []),
        now.editable.reconfigure([EditorState.readOnly.of(!editable), EditorView.editable.of(editable)]),
        now.name.reconfigure(EditorView.contentAttributes.of({ "aria-label": name })),
      ],
    });
  }, [kind, wraps, editable, name]);

  // Keyed on what the marks say, so a render handing over an equal list reconfigures nothing.
  const noted = JSON.stringify(notes ?? []);
  useEffect(() => {
    view.current?.dispatch({ effects: slots.current.notes.reconfigure(noteMarks(JSON.parse(noted) as CodeNoteMark[])) });
  }, [noted]);

  // Plain keys stay in the editor: Bridge's single-key shortcuts listen on the window and would read the host as no text field.
  // On the host, in the bubble phase, so the editor has heard the key by the time it stops. The menu key, and Shift F10,
  // offer a note on the selection, at its end.
  useEffect(() => {
    const el = host.current;
    if (el === null) return;
    const keep = (event: KeyboardEvent) => {
      const at = view.current;
      if (at !== null && noting.current !== undefined && (event.key === "ContextMenu" || (event.key === "F10" && event.shiftKey))) {
        event.preventDefault();
        const { from, to, head } = at.state.selection.main;
        const box = at.coordsAtPos(head) ?? el.getBoundingClientRect();
        const lines = linesOf(at, from, to);
        noting.current({ x: box.left, y: box.bottom }, lines, drawnOf(at, lines));
      }
      if (!event.metaKey && !event.ctrlKey) event.stopPropagation();
    };
    // A right press notes the selection, and with nothing selected the line under the pointer.
    const menu = (event: MouseEvent) => {
      const at = view.current;
      if (at === null || noting.current === undefined) return;
      event.preventDefault();
      const { from, to } = at.state.selection.main;
      const under = from === to ? at.posAtCoords({ x: event.clientX, y: event.clientY }) : null;
      const lines = under === null ? linesOf(at, from, to) : linesOf(at, under, under);
      noting.current({ x: event.clientX, y: event.clientY }, lines, drawnOf(at, lines));
    };
    el.addEventListener("keydown", keep);
    el.addEventListener("contextmenu", menu);
    return () => {
      el.removeEventListener("keydown", keep);
      el.removeEventListener("contextmenu", menu);
    };
  }, []);

  return <div ref={host} className="armada-code-editor" role="group" aria-label={`Editor ${name}`} data-keeps-keys="" />;
}

/**
 * `words` selected where they first stand in the editor `element` is the host of, as a pointer drag leaves them. For a walk,
 * which cannot reach into the shadow root. False where `element` is no editor or the words are not in it.
 */
export function selectInEditor(element: HTMLElement, words: string): boolean {
  const inner = element.shadowRoot?.querySelector<HTMLElement>(".cm-editor");
  const at = inner === null || inner === undefined ? null : EditorView.findFromDOM(inner);
  if (at === null) return false;
  const from = at.state.doc.toString().indexOf(words);
  if (from < 0) return false;
  at.focus();
  at.dispatch({ selection: { anchor: from, head: from + words.length }, scrollIntoView: true, userEvent: "select.pointer" });
  return true;
}

/**
 * Words typed into the editor `element` is the host of: at the end of the text, or at the end of line N where they open
 * with `@N `. For a walk, which finds the host and cannot reach what a shadow root holds. False where `element` is no editor.
 */
export function typeIntoEditor(element: HTMLElement, said: string): boolean {
  const inner = element.shadowRoot?.querySelector<HTMLElement>(".cm-editor");
  const at = inner === null || inner === undefined ? null : EditorView.findFromDOM(inner);
  if (at === null) return false;
  at.focus();
  const line = /^@(\d+) /.exec(said);
  const words = line === null ? said : said.slice(line[0].length);
  const end = line === null ? at.state.doc.length : at.state.doc.line(Math.min(Number(line[1]), at.state.doc.lines)).to;
  at.dispatch({ changes: { from: end, insert: words }, selection: { anchor: end + words.length }, userEvent: "input.type" });
  return true;
}
