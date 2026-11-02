// What a `mock` is made of, apart from React: the HTML as nodes of the frame's own document, the part an element stands
// for, and where a point or a box lands in the frame. **Nothing here knows an app's classes or tokens**: a part is named by
// the `data-part` the agent wrote, or by its words, and the look is whatever sheets the host resolved.

import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";

import { MOCK_ICONS } from "./mock-icons";

/** Dropped from the HTML: nothing that runs, loads or reaches outside the frame is drawn. */
const DROPPED = "script,style,link,iframe,frame,object,embed,base,meta,noscript,template";
/** Attributes that carry an address; a fragment alone is kept for an SVG's `use`. */
const ADDRESSES = new Set(["src", "srcset", "href", "xlink:href", "action", "formaction", "poster", "background", "data"]);
/** The size of a glyph with no `data-size`, as the two sizes the icon contract names the larger of. */
const GLYPH = 16;

function glyph(source: Document, name: string, size: number): Element | undefined {
  const Icon = MOCK_ICONS[name];
  if (Icon === undefined) return undefined;
  const parsed = new DOMParser().parseFromString(renderToStaticMarkup(createElement(Icon, { size, strokeWidth: 2, "aria-hidden": true })), "text/html");
  const svg = parsed.body.firstElementChild;
  return svg === null ? undefined : (source.importNode(svg, true) as Element);
}

/**
 * The HTML as nodes of `target`. Parsed inert, cleaned, and `<i data-icon>` swapped for its glyph, so what reaches the frame is
 * markup only: no script, no handler, no address, no inline style.
 */
export function mockNodes(target: Document, html: string): Node[] {
  const source = new DOMParser().parseFromString(`<!doctype html><body>${html}`, "text/html");
  for (const one of [...source.querySelectorAll(DROPPED)]) one.remove();
  for (const one of source.querySelectorAll("*")) {
    for (const attribute of [...one.attributes]) {
      const name = attribute.name.toLowerCase();
      if (name.startsWith("on") || name === "style" || (ADDRESSES.has(name) && !attribute.value.trim().startsWith("#"))) one.removeAttribute(attribute.name);
    }
  }
  for (const one of [...source.querySelectorAll("i[data-icon]")]) {
    const size = Number.parseInt(one.getAttribute("data-size") ?? "", 10);
    const drawn = glyph(source, one.getAttribute("data-icon") ?? "", Number.isFinite(size) ? size : GLYPH);
    if (drawn === undefined) continue;
    for (const attribute of [...one.attributes]) if (attribute.name !== "data-icon" && attribute.name !== "data-size") drawn.setAttribute(attribute.name, attribute.value);
    one.replaceWith(drawn);
  }
  return [...source.body.childNodes].map((one) => target.importNode(one, true));
}

const hasOwnText = (one: Element) => [...one.childNodes].some((child) => child.nodeType === Node.TEXT_NODE && (child.textContent ?? "").trim() !== "");
const words = (one: Element) => (one.textContent ?? "").replace(/\s+/g, " ").trim();
const LABEL_LENGTH = 40;

/** The part an element stands for: the nearest one carrying `data-part`, else the nearest with text of its own or in it. */
export function partOf(from: Element | null, body: Element): Element | undefined {
  const inside = (one: Element | null): one is Element => one !== null && one !== body && one !== body.parentElement;
  for (let at = from; inside(at); at = at.parentElement) if (at.hasAttribute("data-part")) return at;
  for (let at = from; inside(at); at = at.parentElement) if (words(at) !== "") return at;
  return undefined;
}

/**
 * Where a part is found again: its `data-part` (with its place among the same name when the name repeats), else its path from the body.
 * A path is a selector the agent can run on the HTML it wrote.
 */
export function partKey(one: Element, body: Element): string {
  const named = one.getAttribute("data-part");
  if (named !== null) {
    const same = [...body.querySelectorAll("[data-part]")].filter((other) => other.getAttribute("data-part") === named);
    return same.length > 1 ? `${named}[${String(same.indexOf(one) + 1)}]` : named;
  }
  const path: string[] = [];
  for (let at: Element | null = one; at !== null && at !== body; at = at.parentElement) {
    path.unshift(`${at.tagName.toLowerCase()}:nth-child(${String([...(at.parentElement?.children ?? [])].indexOf(at) + 1)})`);
  }
  return ["body", ...path].join(" > ");
}

/** What a part is called: the name the agent gave it, else its words, cut short. */
export function partName(one: Element): string {
  const named = one.getAttribute("data-part");
  if (named !== null) return named;
  const said = words(one);
  return said.length > LABEL_LENGTH ? `${said.slice(0, LABEL_LENGTH).trimEnd()}…` : said;
}

/** The element a key names, if the frame still holds it. */
export function elementOfKey(doc: Document, key: string): Element | undefined {
  if (key.startsWith("body")) {
    try {
      return doc.querySelector(key) ?? undefined;
    } catch {
      return undefined;
    }
  }
  const repeat = /^(.*)\[(\d+)\]$/.exec(key);
  const name = repeat?.[1] ?? key;
  const same = [...doc.querySelectorAll("[data-part]")].filter((other) => other.getAttribute("data-part") === name);
  return same[repeat === null ? 0 : Number(repeat[2]) - 1];
}

export type MockPart = { id: string; label: string };

const idOf = (node: string, key: string) => `${node}#${key}`;
/** The scene node a part id belongs to, and the key inside its frame. */
export function splitPart(id: string): { node: string; key: string } | undefined {
  const at = id.indexOf("#");
  return at < 0 ? undefined : { node: id.slice(0, at), key: id.slice(at + 1) };
}

function partFor(node: string, el: Element, body: Element): MockPart {
  return { id: idOf(node, partKey(el, body)), label: partName(el) };
}

/** A point on the screen, as a point of the frame: undone of the canvas's zoom, which scales the frame's box and not its pixels. */
function inFrame(frame: HTMLIFrameElement, x: number, y: number): { x: number; y: number } {
  const box = frame.getBoundingClientRect();
  const scale = frame.offsetWidth === 0 ? 1 : box.width / frame.offsetWidth;
  return { x: (x - box.left) / scale, y: (y - box.top) / scale };
}

/** The part under a press at a screen point, or `undefined` where it lands on the frame's bare ground. */
export function partAt(frame: HTMLIFrameElement, node: string, x: number, y: number): MockPart | undefined {
  const doc = frame.contentDocument;
  if (doc === null) return undefined;
  const at = inFrame(frame, x, y);
  const part = partOf(doc.elementFromPoint(at.x, at.y), doc.body);
  return part === undefined ? undefined : partFor(node, part, doc.body);
}

/** Every part a screen box touches: the parts of the elements it overlaps, once each. */
export function partsIn(frame: HTMLIFrameElement, node: string, box: { left: number; right: number; top: number; bottom: number }): MockPart[] {
  const doc = frame.contentDocument;
  if (doc === null) return [];
  const from = inFrame(frame, box.left, box.top);
  const to = inFrame(frame, box.right, box.bottom);
  const found = new Map<string, MockPart>();
  for (const one of doc.body.querySelectorAll("*")) {
    if (!one.hasAttribute("data-part") && !hasOwnText(one)) continue;
    const at = one.getBoundingClientRect();
    if (at.right < from.x || at.left > to.x || at.bottom < from.y || at.top > to.y) continue;
    const part = partOf(one, doc.body);
    if (part === undefined) continue;
    const made = partFor(node, part, doc.body);
    found.set(made.id, made);
  }
  return [...found.values()];
}
