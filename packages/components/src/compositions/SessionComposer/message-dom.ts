import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";

import { InlineTag } from "./InlineTag";
import type { ComposerTag } from "./InlineTag";

/** Where a tag stands in a message's text. The tags themselves are listed beside it, in order. */
export const CHIP = "￼";

/**
 * The message is markdown and the box only draws it. **A run of lines behind `> ` is one quote block and a
 * run behind `- ` or `* ` one list**, each line an element of its own with the marker left out; every
 * other line is a `div`. `readDom` gives the markers back, so what is kept and sent is the text it always was.
 */
type Kind = "quote" | "list";
const BLOCK: Record<Kind, { tag: string; line: string; marker: string }> = {
  quote: { tag: "blockquote", line: "div", marker: "> " },
  list: { tag: "ul", line: "li", marker: "- " },
};

const kindOf = (node: Node): Kind | undefined => {
  const kind = node instanceof HTMLElement ? node.dataset.block : undefined;
  return kind === "quote" || kind === "list" ? kind : undefined;
};

/** What starts a drawn line: `> ` for a quote, `- ` or `* ` for an item. A lone `>` is an empty quote line in text that arrives. */
const markerOf = (line: string): { kind: Kind; content: string } | undefined => {
  if (line.startsWith("> ")) return { kind: "quote", content: line.slice(2) };
  if (line === ">") return { kind: "quote", content: "" };
  if (/^[-*] /.test(line)) return { kind: "list", content: line.slice(2) };
  return undefined;
};

/** Each line's marker, none inside a ``` fence: a pasted diff's `- ` lines are code, not a list. */
const markersOf = (lines: readonly string[]): ({ kind: Kind; content: string } | undefined)[] => {
  let fenced = false;
  return lines.map((line) => {
    if (line.trimStart().startsWith("```")) {
      fenced = !fenced;
      return undefined;
    }
    return fenced ? undefined : markerOf(line);
  });
};

/** Whether a node is something the box holds whole: a tag, or a markdown link. */
const isAtom = (node: Node): node is HTMLElement => node instanceof HTMLElement && (node.dataset.tagKind !== undefined || node.dataset.mdHref !== undefined);

/** The message as the box holds it: its text with a CHIP at each tag, and the tags in that order. */
export function readDom(root: Node): { text: string; tags: ComposerTag[] } {
  const tags: ComposerTag[] = [];
  const lines: string[] = [];
  let current: string | null = null;
  const end = () => {
    if (current !== null) lines.push(current);
    current = null;
  };
  const walk = (node: Node, prefix: string) => {
    node.childNodes.forEach((child) => {
      if (child.nodeType === Node.TEXT_NODE) {
        (child as Text).data.split("\n").forEach((part, index) => {
          if (index > 0) {
            lines.push(current ?? prefix);
            current = null;
          }
          if (part !== "") current = (current ?? prefix) + part;
        });
      } else if (child instanceof HTMLElement) {
        if (child.dataset.tagKind !== undefined) {
          current = (current ?? prefix) + CHIP;
          tags.push({
            kind: child.dataset.tagKind as ComposerTag["kind"],
            id: child.dataset.tagId ?? "",
            title: child.dataset.tagTitle ?? "",
            ...(child.dataset.tagUrl === undefined ? {} : { url: child.dataset.tagUrl }),
          });
        } else if (child.dataset.mdHref !== undefined) {
          current = (current ?? prefix) + `[${child.dataset.mdWords ?? ""}](${child.dataset.mdHref})`;
        } else if (child.tagName === "BR") {
          // A break that ends its line after words is where the browser keeps the caret, not a line.
          if (child !== child.parentNode?.lastChild || current === null) {
            lines.push(current ?? prefix);
            current = null;
          }
        } else if (child.tagName === "BLOCKQUOTE" || child.tagName === "UL") {
          end();
          const kind = kindOf(child);
          walk(child, kind === undefined ? prefix : BLOCK[kind].marker);
          end();
        } else if (child.tagName === "DIV" || child.tagName === "P" || child.tagName === "LI") {
          end();
          const before = lines.length;
          walk(child, prefix);
          end();
          if (lines.length === before) lines.push(prefix);
        } else {
          walk(child, prefix);
        }
      }
    });
  };
  walk(root, "");
  end();
  return { text: lines.join("\n"), tags };
}

/**
 * The message as it is sent: each tag written out as `@title` at its place, and a blank line where a quote
 * or a list ends in a line of words. The box draws that line apart from the block; markdown would read
 * it as the block's own lazy continuation, and the thread would draw what the box did not.
 */
export function sentText(text: string, tags: readonly ComposerTag[]): string {
  let at = 0;
  const written = text.replace(new RegExp(CHIP, "g"), () => `@${tags[at++]?.title ?? ""}`);
  const lines = written.split("\n");
  const marks = markersOf(lines);
  return lines.flatMap((line, index) => (marks[index] !== undefined && lines[index + 1] !== undefined && lines[index + 1] !== "" && marks[index + 1] === undefined ? [line, ""] : [line])).join("\n");
}

function chipNode(tag: ComposerTag): Node {
  const holder = document.createElement("template");
  holder.innerHTML = renderToStaticMarkup(createElement(InlineTag, { tag, atomic: true }));
  return holder.content.firstChild!;
}

/** Draws a message into the box, replacing what was there. */
export function writeDom(root: HTMLElement, text: string, tags: readonly ComposerTag[]): void {
  root.replaceChildren();
  if (text === "") return;
  let at = 0;
  let block: HTMLElement | null = null;
  let blockKind: Kind | undefined;
  const lines = text.split("\n");
  const marks = markersOf(lines);
  lines.forEach((line, index) => {
    const marked = marks[index];
    const el = document.createElement(marked === undefined ? "div" : BLOCK[marked.kind].line);
    (marked?.content ?? line).split(CHIP).forEach((piece, index) => {
      if (index > 0) {
        const tag = tags[at++];
        if (tag !== undefined) el.append(chipNode(tag));
      }
      if (piece !== "") el.append(document.createTextNode(piece));
    });
    if (!el.hasChildNodes()) el.append(document.createElement("br"));
    if (marked === undefined) {
      root.append(el);
      block = null;
      return;
    }
    if (block === null || blockKind !== marked.kind) {
      block = document.createElement(BLOCK[marked.kind].tag);
      block.dataset.block = marked.kind;
      root.append(block);
      blockKind = marked.kind;
    }
    block.append(el);
  });
  decorate(root);
}

/** A tag, as a node the box can hold. */
export const tagNode = chipNode;

/** What stands before the caret, tags as spaces, and the text node the caret is in. */
export function beforeCaret(root: HTMLElement): { head: string; node: Text | null; offset: number } | undefined {
  const selection = window.getSelection();
  if (selection === null || selection.rangeCount === 0) return undefined;
  const range = selection.getRangeAt(0);
  if (!range.collapsed || !root.contains(range.startContainer)) return undefined;
  const before = document.createRange();
  before.selectNodeContents(root);
  before.setEnd(range.startContainer, range.startOffset);
  const holder = document.createElement("div");
  holder.append(before.cloneContents());
  const head = readDom(holder).text.replace(new RegExp(CHIP, "g"), " ");
  if (range.startContainer.nodeType === Node.TEXT_NODE) return { head, node: range.startContainer as Text, offset: range.startOffset };
  // A caret at the end of a line the box was drawn with sits between nodes, after the text node it follows.
  const prior = range.startContainer.childNodes[range.startOffset - 1];
  return prior?.nodeType === Node.TEXT_NODE ? { head, node: prior as Text, offset: (prior as Text).length } : { head, node: null, offset: 0 };
}

function setCaret(node: Node, offset: number): void {
  const range = document.createRange();
  range.setStart(node, offset);
  range.collapse(true);
  const selection = window.getSelection();
  selection?.removeAllRanges();
  selection?.addRange(range);
}

/** The caret at the end of the box: at the end of its last line, before the break that holds an empty one open. */
export function caretToEnd(root: HTMLElement): void {
  let node: Node = root;
  while (node.lastChild !== null && node.lastChild.nodeType !== Node.TEXT_NODE && !isAtom(node.lastChild) && node.lastChild.nodeName !== "BR") node = node.lastChild;
  const last = node.lastChild;
  if (last?.nodeType === Node.TEXT_NODE) setCaret(last, (last as Text).length);
  else setCaret(node, last?.nodeName === "BR" ? node.childNodes.length - 1 : node.childNodes.length);
}

/** How much of the message a node stands for, as `readDom` counts it. */
function sizeOf(node: Node): number {
  if (node.nodeType === Node.TEXT_NODE) return (node as Text).length;
  if (!(node instanceof HTMLElement)) return 0;
  if (node.dataset.tagKind !== undefined) return 1;
  if (node.dataset.mdHref !== undefined) return `[${node.dataset.mdWords ?? ""}](${node.dataset.mdHref})`.length;
  let size = 0;
  node.childNodes.forEach((child) => (size += sizeOf(child)));
  return size;
}

/** The caret `offset` characters into the message as it was drawn by `writeDom`. */
function placeCaret(root: HTMLElement, offset: number): void {
  const lines: { el: HTMLElement; marker: number }[] = [];
  root.childNodes.forEach((child) => {
    if (!(child instanceof HTMLElement)) return;
    if (kindOf(child) === undefined) lines.push({ el: child, marker: 0 });
    else child.childNodes.forEach((line) => line instanceof HTMLElement && lines.push({ el: line, marker: 2 }));
  });
  let start = 0;
  for (const { el, marker } of lines) {
    const length = marker + sizeOf(el);
    if (offset <= start + length) {
      let left = Math.max(offset - start - marker, 0);
      for (const [index, child] of Array.from(el.childNodes).entries()) {
        const size = sizeOf(child);
        if (child.nodeName === "BR") return setCaret(el, index);
        if (child.nodeType === Node.TEXT_NODE) {
          if (left <= size) return setCaret(child, left);
        } else if (left <= size) {
          if (isAtom(child)) return setCaret(el, left === 0 ? index : index + 1);
          // A drawn link holds one text node.
          const words = child.firstChild;
          return setCaret(words ?? child, Math.min(left, words instanceof Text ? words.length : 0));
        }
        left -= size;
      }
      return setCaret(el, el.childNodes.length);
    }
    start += length + 1;
  }
  caretToEnd(root);
}

/** Where the caret stands in the message, in characters. */
export function caretOffset(root: HTMLElement): number {
  return beforeCaret(root)?.head.length ?? readDom(root).text.length;
}

/** Whether the lines the text marks as quote or list are drawn as such, and no others are. */
export function drawnAsText(root: HTMLElement): boolean {
  const marked = markersOf(readDom(root).text.split("\n")).filter((one) => one !== undefined).length;
  return marked === root.querySelectorAll(":scope > [data-block] > *").length;
}

/** The box drawn again from what it reads, the caret where it stood. */
export function redraw(root: HTMLElement, offset: number): void {
  const { text, tags } = readDom(root);
  writeDom(root, text, tags);
  placeCaret(root, offset);
}

// A quote or a list line, and the keys that leave it.

type Line = { line: HTMLElement; block: HTMLElement; kind: Kind };

/** The quote or list line the caret is in, when it is a caret and not a selection. */
function lineOf(root: HTMLElement): Line | undefined {
  const selection = window.getSelection();
  if (selection === null || selection.rangeCount === 0 || !selection.isCollapsed) return undefined;
  let node: Node | null = selection.anchorNode;
  if (node === null || !root.contains(node)) return undefined;
  while (node !== null && node !== root) {
    const parent: HTMLElement | null = node.parentElement;
    const kind = parent === null ? undefined : kindOf(parent);
    if (parent !== null && kind !== undefined && node instanceof HTMLElement) return { line: node, block: parent, kind };
    node = parent;
  }
  return undefined;
}

const holds = (range: Range): boolean => range.toString() !== "" || range.cloneContents().querySelector("[data-tag-kind],[data-md-href]") !== null;
const empty = (line: HTMLElement): boolean => (line.textContent ?? "") === "" && line.querySelector("[data-tag-kind],[data-md-href]") === null;

function caretIs(line: HTMLElement, where: "start" | "end"): boolean {
  const caret = window.getSelection()!.getRangeAt(0);
  const span = document.createRange();
  if (where === "start") {
    span.setStart(line, 0);
    span.setEnd(caret.startContainer, caret.startOffset);
  } else {
    span.setStart(caret.startContainer, caret.startOffset);
    span.setEnd(line, line.childNodes.length);
  }
  return !holds(span);
}

const held = (el: HTMLElement) => {
  if (!el.hasChildNodes() || (empty(el) && el.querySelector("br") === null)) el.replaceChildren(document.createElement("br"));
};

/** The line as a plain one where it stands in its block, the block split around it. */
function leave({ line, block, kind }: Line): void {
  const plain = document.createElement("div");
  plain.append(...Array.from(line.childNodes));
  held(plain);
  const after = Array.from(block.children).slice(Array.from(block.children).indexOf(line) + 1);
  line.remove();
  block.after(plain);
  if (after.length > 0) {
    const rest = document.createElement(BLOCK[kind].tag);
    rest.dataset.block = kind;
    rest.append(...after);
    plain.after(rest);
  }
  if (block.children.length === 0) block.remove();
  setCaret(plain, 0);
}

/** The caret's line split in two at the caret: the words after it start a new line of the same block. */
function split({ line }: Line): void {
  const rest = document.createRange();
  const caret = window.getSelection()!.getRangeAt(0);
  rest.setStart(caret.startContainer, caret.startOffset);
  rest.setEnd(line, line.childNodes.length);
  const next = document.createElement(line.tagName);
  next.append(rest.extractContents());
  line.after(next);
  held(line);
  held(next);
  setCaret(next, 0);
}

/**
 * Enter and Shift+Enter in a quote or a list; false leaves the key to the box. **In a list, Enter starts
 * the next item, and on an empty item ends the list.** In a quote, Enter at the end of the last line
 * leaves it for a plain line under it (an empty last line goes back to plain), and anywhere else is the
 * box's own. Shift+Enter starts another line of the same block. **Ctrl or Cmd with Enter is the box's**, which sends.
 */
export function enterInBlock(root: HTMLElement, shift: boolean): boolean {
  const at = lineOf(root);
  if (at === undefined) return false;
  if (at.kind === "list") {
    if (empty(at.line)) leave(at);
    else split(at);
    return true;
  }
  if (shift) {
    split(at);
    return true;
  }
  if (at.line !== at.block.lastElementChild || !caretIs(at.line, "end")) return false;
  if (empty(at.line)) {
    leave(at);
    return true;
  }
  const plain = document.createElement("div");
  held(plain);
  at.block.after(plain);
  setCaret(plain, 0);
  return true;
}

/** Backspace at the start of a quote or list line makes it a plain line. */
export function backspaceInBlock(root: HTMLElement): boolean {
  const at = lineOf(root);
  if (at === undefined || !caretIs(at.line, "start")) return false;
  leave(at);
  return true;
}

// Links, drawn as the words are typed.

const URL_ADDRESS = /^https?:\/\/[^\s<>/][^\s<>]*$/;
const KNOWN = new Set(["rs", "ts", "tsx", "js", "jsx", "mjs", "json", "toml", "md", "mdx", "css", "html", "yml", "yaml", "sh", "py", "go", "lock", "txt", "sql", "svg"]);
const MARKDOWN_LINK = /\[([^\]\n]+)\]\((https?:\/\/[^\s)]+)\)/g;

/**
 * A repository path, the way `@` writes one or as typed: `crates/fleet/src/servers.rs`, `servers.rs:60`,
 * `@src/`. A name with no folder needs an extension a repository holds, so `e.g.` and `v1.2` stay words.
 */
function isPath(core: string): boolean {
  const at = core.startsWith("@");
  const found = /^((?:[\w.-]+\/)*[\w.-]+)(\/?)(?::\d+(?:[:-]\d+)?)?$/.exec(at ? core.slice(1) : core);
  if (found === null || /^\.+$/.test(found[1]!)) return false;
  const path = found[1]!;
  const extension = /\.([A-Za-z][A-Za-z0-9]{0,7})$/.exec(path)?.[1]?.toLowerCase();
  const folder = path.includes("/");
  if (found[2] === "/") return at;
  if (at) return folder || (extension !== undefined && KNOWN.has(extension));
  return folder ? extension !== undefined : extension !== undefined && KNOWN.has(extension);
}

type Link = "web" | "file";
const classify = (core: string): Link | null => (URL_ADDRESS.test(core) ? "web" : isPath(core) ? "file" : null);

/** The part of a whitespace-delimited word that is the address: not the bracket or quote before it, nor the stop or comma after. */
function coreOf(word: string): [number, number] {
  let from = 0;
  let to = word.length;
  while (from < to && "(\"'`<[".includes(word[from]!)) from += 1;
  while (to > from && ".,;:!?\"'`>]".includes(word[to - 1]!)) to -= 1;
  while (to > from && word[to - 1] === ")" && !word.slice(from, to).includes("(")) to -= 1;
  return [from, to];
}

function mdNode(words: string, href: string): HTMLElement {
  const node = document.createElement("span");
  node.className = "armada-prose__link armada-session-composer__link";
  node.contentEditable = "false";
  node.title = href;
  node.dataset.mdWords = words;
  node.dataset.mdHref = href;
  node.textContent = words;
  return node;
}

/** Whether anything is drawn after this text in the box, so that its last word is closed by it. */
function followed(node: Node, root: Node): boolean {
  for (let at: Node | null = node; at !== null && at !== root; at = at.parentNode) {
    for (let next = at.nextSibling; next !== null; next = next.nextSibling) if (next.nodeType !== Node.TEXT_NODE || (next as Text).length > 0) return true;
  }
  return false;
}

function textNodes(root: HTMLElement): Text[] {
  const found: Text[] = [];
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT, {
    acceptNode: (node) => (node.parentElement?.closest("[data-link],[data-tag-kind],[data-md-href]") ? NodeFilter.FILTER_REJECT : NodeFilter.FILTER_ACCEPT),
  });
  for (let node = walker.nextNode(); node !== null; node = walker.nextNode()) found.push(node as Text);
  return found;
}

/**
 * Draws a web address, a repository path and a markdown link as links, without changing a character of the
 * text: an address or path is wrapped where it stands, a markdown link becomes a node that holds the whole.
 * **A word is drawn once something closes it** (a space, a stop, a line end, a tag after it) and never while the
 * caret is in it, so the caret does not move under the person typing. A drawn link whose text stops being an
 * address is undrawn.
 */
export function decorate(root: HTMLElement): void {
  root.querySelectorAll<HTMLElement>("[data-link]").forEach((span) => {
    if (classify(span.textContent ?? "") !== span.dataset.link) {
      const parent = span.parentNode;
      span.replaceWith(...Array.from(span.childNodes));
      parent?.normalize();
    }
  });
  const selection = window.getSelection();
  const caret = selection !== null && selection.rangeCount > 0 && selection.isCollapsed && root.contains(selection.anchorNode) ? { node: selection.anchorNode, offset: selection.anchorOffset } : undefined;

  for (const node of textNodes(root)) {
    const matches = Array.from(node.data.matchAll(MARKDOWN_LINK)).reverse();
    for (const found of matches) {
      const from = found.index;
      const to = from + found[0].length;
      const inside = caret?.node === node && caret.offset > from && caret.offset < to;
      if (inside) continue;
      const after = caret?.node === node && caret.offset === to;
      const range = document.createRange();
      range.setStart(node, from);
      range.setEnd(node, to);
      range.deleteContents();
      const draw = mdNode(found[1]!, found[2]!);
      range.insertNode(draw);
      if (after) {
        if (!(draw.nextSibling instanceof Text)) draw.after(document.createTextNode(""));
        setCaret(draw.nextSibling!, 0);
      }
    }
  }

  for (const node of textNodes(root)) {
    const closedAtEnd = followed(node, root);
    const words = Array.from(node.data.matchAll(/\S+/g)).reverse();
    for (const word of words) {
      const [from, to] = coreOf(word[0]);
      const start = word.index + from;
      const end = word.index + to;
      const stop = word.index + word[0].length;
      if (from >= to) continue;
      const touched = caret?.node === node && caret.offset >= start && caret.offset <= stop;
      const closed = stop < node.length || closedAtEnd || to < word[0].length;
      const kind = classify(node.data.slice(start, end));
      if (touched || !closed || kind === null) continue;
      const range = document.createRange();
      range.setStart(node, start);
      range.setEnd(node, end);
      const link = document.createElement("span");
      link.className = kind === "web" ? "armada-prose__link" : "armada-prose__link armada-session-composer__path";
      link.dataset.link = kind;
      link.title = node.data.slice(start, end);
      range.surroundContents(link);
    }
  }
}

/** The address a link in the box opens, for a web link; a path opens nothing. */
export function addressOf(node: Element | null): string | undefined {
  const link = node?.closest<HTMLElement>("[data-link],[data-md-href]");
  if (link === null || link === undefined) return undefined;
  return link.dataset.mdHref ?? (link.dataset.link === "web" ? (link.textContent ?? undefined) : undefined);
}
