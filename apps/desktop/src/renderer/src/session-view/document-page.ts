// A Job's document, read: the HTML the Job wrote, made safe to draw inside Bridge's own page. **Sealed by construction.** The text is
// parsed by `DOMParser`, which runs no script and loads nothing, and the page is rebuilt element by element from an allow list, so no
// script, handler, frame, form, image or `style` attribute survives. An inline `svg` is rebuilt the same way, from shapes and text with presentation attributes, so a chart
// can be drawn and nothing in it can load or run. Its CSS is read rule by rule, kept only where it holds no address,
// and every selector is put under the page's own root with its classes and ids renamed, so it styles the page and nothing of Bridge's
// and Bridge's classes style nothing in it. Bridge's CSP refuses inline script and handlers besides. The kit is the one link kept.

import { onOneLine } from "@armada/components";

import { KIT_CSS, KIT_HREF } from "./document-kit";

/** The class of the element a page is drawn in: every selector the page writes is put under it. */
export const PAGE_ROOT = "armada-doc__page";

export type PageSketch = { title: string; scene: unknown };

export type Page = {
  title: string;
  /** The first paragraph after the title: the conclusion, as the Landing shows it. */
  lede: string;
  /** The page's h2s, in order, for the contents. */
  sections: readonly { id: string; title: string }[];
  /** The cleaned page, detached. Cloned into the layer; never drawn as it is. */
  root: HTMLElement;
  /** The page's CSS, kit first, every rule under `PAGE_ROOT`. */
  css: string;
  sketches: readonly PageSketch[];
};

const ALLOWED = new Set(
  "a abbr aside b blockquote br caption cite code col colgroup dd del details dfn div dl dt em figcaption figure footer h1 h2 h3 h4 h5 h6 header hr i ins kbd li main mark nav ol p pre q s samp section small span strong sub summary sup table tbody td tfoot th thead time tr u ul var".split(" "),
);
/** Dropped with everything inside them. Any other tag that is not allowed is dropped and what it held is kept. */
const DROPPED = new Set("script style link meta title head iframe frame frameset object embed applet form input button textarea select option math img picture source audio video canvas template noscript base".split(" "));
const KEPT_ATTRIBUTES = new Set(["colspan", "rowspan", "scope", "headers", "datetime", "open", "start", "lang", "title", "abbr"]);

const SVG_NS = "http://www.w3.org/2000/svg";
/** Shapes, text and grouping, enough for a chart. Anything else inside an `svg` is dropped with what it holds; an `a` without an address is unwrapped, and one with an address is dropped. */
const SVG_ALLOWED = new Set("svg g path rect circle ellipse line polyline polygon text tspan title desc defs clipPath".split(" "));
const SVG_ATTRIBUTES = new Set(
  (
    "viewBox preserveAspectRatio x y width height cx cy r rx ry x1 y1 x2 y2 points d transform dx dy textLength lengthAdjust clipPathUnits role aria-label aria-hidden " +
    "fill fill-opacity fill-rule stroke stroke-width stroke-opacity stroke-linecap stroke-linejoin stroke-dasharray stroke-dashoffset stroke-miterlimit opacity clip-path clip-rule " +
    "font-family font-size font-weight font-style letter-spacing text-anchor text-decoration dominant-baseline alignment-baseline baseline-shift visibility display"
  ).split(" "),
);

const OUTSIDE = ":not(.armada-doc__sketch-mount *)";

const squeeze = (words: string) => words.replace(/\s+/g, " ").trim();
const renamed = (selector: string) =>
  selector.replace(/\.(-?[_a-zA-Z][\w-]*)/g, ".d-$1").replace(/#(-?[_a-zA-Z][\w-]*)/g, "#d-$1");

/** One selector of a rule, put under the page's root. `body`, `html` and `:root` are the root itself. */
function scoped(selector: string): string {
  const one = renamed(selector.trim());
  const root = /^(?:html\s+)?(?:body|html|:root)(?=$|[\s>+~:.[])/.exec(one);
  const under = root === null ? `.${PAGE_ROOT} ${one}` : `.${PAGE_ROOT}${one.slice(root[0].length)}`;
  // Not inside a sketch: the page's `p` or `table` styles the page, and Bridge's sketch is drawn by Bridge.
  const pseudo = under.search(/::/);
  return pseudo < 0 ? `${under}${OUTSIDE}` : `${under.slice(0, pseudo)}${OUTSIDE}${under.slice(pseudo)}`;
}

function rulesOf(rules: CSSRuleList): string[] {
  const out: string[] = [];
  for (const rule of Array.from(rules)) {
    if (rule instanceof CSSStyleRule) {
      const body = rule.style.cssText;
      // An address in a declaration is a request the page could make, and a fixed or sticky box could cover Bridge.
      if (/url\(|image-set\(|expression\(|@import/i.test(body)) continue;
      const selectors = rule.selectorText.split(/,(?![^(]*\))/).map(scoped).join(", ");
      out.push(`${selectors} { ${body} }`);
    } else if (rule instanceof CSSMediaRule) {
      const inner = rulesOf(rule.cssRules);
      if (inner.length > 0) out.push(`@media ${rule.conditionText} { ${inner.join(" ")} }`);
    }
  }
  return out;
}

/** CSS from the page, as rules that reach only the page. A rule that does not parse is gone, as a browser drops it. */
export function scopeCss(css: string): string {
  const sheet = new CSSStyleSheet();
  sheet.replaceSync(css.replace(/@import[^;]*;/gi, ""));
  return rulesOf(sheet.cssRules).join("\n");
}

type Gathered = { sketches: PageSketch[]; headings: number };

function sketchOf(source: Element, into: Gathered): HTMLElement {
  const title = source.getAttribute("title") ?? "Sketch";
  const raw = source.querySelector('script[type="application/json"]')?.textContent ?? source.getAttribute("scene") ?? "";
  let scene: unknown;
  try {
    scene = JSON.parse(raw);
  } catch {
    scene = undefined;
  }
  const figure = document.createElement("figure");
  figure.className = "armada-doc__sketch";
  figure.dataset.sketch = String(into.sketches.length);
  figure.dataset.title = title;
  const mount = document.createElement("div");
  mount.className = "armada-doc__sketch-mount";
  const caption = document.createElement("figcaption");
  caption.textContent = title;
  figure.append(mount, caption);
  into.sketches.push({ title, scene });
  return figure;
}

function copy(from: Element, into: Gathered): HTMLElement {
  const name = from.tagName.toLowerCase();
  const one = document.createElement(name);
  for (const { name: attribute, value } of Array.from(from.attributes)) {
    if (attribute === "class") one.className = value.split(/\s+/).filter((x) => x !== "").map((x) => `d-${x}`).join(" ");
    else if (attribute === "id") one.id = `d-${value}`;
    else if (attribute === "href" && name === "a") {
      if (value.startsWith("#")) one.setAttribute("href", `#d-${value.slice(1)}`);
      else if (/^https?:\/\//i.test(value)) one.setAttribute("href", value);
    } else if (KEPT_ATTRIBUTES.has(attribute)) one.setAttribute(attribute, value);
  }
  if (name === "h2" && one.id === "") {
    into.headings += 1;
    one.id = `d-section-${String(into.headings)}`;
  }
  // A row can be reached from the keyboard, to note it.
  if (name === "tr" && from.closest("tbody") !== null) one.tabIndex = 0;
  fill(from, one, into);
  return one;
}

/** A presentation value, kept unless it reaches for an address: only a reference to an `id` in the same chart (`clip-path`) survives, renamed as ids are. */
function svgValue(value: string): string | undefined {
  if (!/url\(|image-set\(|expression\(|javascript:|@import/i.test(value)) return value;
  const local = /^\s*url\(\s*#(-?[_a-zA-Z][\w-]*)\s*\)\s*$/.exec(value);
  return local === null ? undefined : `url(#d-${local[1]!})`;
}

function svgCopy(from: Element): SVGElement {
  const one = document.createElementNS(SVG_NS, from.localName);
  for (const { name, value } of Array.from(from.attributes)) {
    if (name === "class") one.setAttribute("class", value.split(/\s+/).filter((x) => x !== "").map((x) => `d-${x}`).join(" "));
    else if (name === "id") one.setAttribute("id", `d-${value}`);
    else if (SVG_ATTRIBUTES.has(name)) {
      const kept = svgValue(value);
      if (kept !== undefined) one.setAttribute(name, kept);
    }
  }
  svgFill(from, one);
  return one;
}

function svgFill(from: Node, into: Node): void {
  for (const child of Array.from(from.childNodes)) {
    if (child.nodeType === Node.TEXT_NODE) {
      into.appendChild(document.createTextNode(child.textContent ?? ""));
      continue;
    }
    if (!(child instanceof Element)) continue;
    if (child.localName === "a") {
      if (!child.hasAttribute("href") && !child.hasAttribute("xlink:href")) svgFill(child, into);
    }
    else if (child.namespaceURI === SVG_NS && SVG_ALLOWED.has(child.localName)) into.appendChild(svgCopy(child));
  }
}

function fill(from: Node, into: Node, gathered: Gathered): void {
  for (const child of Array.from(from.childNodes)) {
    if (child.nodeType === Node.TEXT_NODE) {
      into.appendChild(document.createTextNode(child.textContent ?? ""));
      continue;
    }
    if (!(child instanceof Element)) continue;
    const name = child.tagName.toLowerCase();
    if (name === "armada-sketch") into.appendChild(sketchOf(child, gathered));
    else if (name === "svg" && child.namespaceURI === SVG_NS) into.appendChild(svgCopy(child));
    else if (DROPPED.has(name)) continue;
    else if (ALLOWED.has(name)) into.appendChild(copy(child, gathered));
    else fill(child, into, gathered);
  }
}

/** The page the HTML is, cleaned, with its title, conclusion, contents and CSS. */
export function readPage(html: string): Page {
  const parsed = new DOMParser().parseFromString(html, "text/html");
  const gathered: Gathered = { sketches: [], headings: 0 };
  const root = document.createElement("div");
  fill(parsed.body, root, gathered);
  const h1 = root.querySelector("h1");
  const title = squeeze(parsed.querySelector("title")?.textContent ?? h1?.textContent ?? "Document");
  const after = (p: Element) => h1 === null || (h1.compareDocumentPosition(p) & Node.DOCUMENT_POSITION_FOLLOWING) !== 0;
  const lede = squeeze([...root.querySelectorAll("p")].find((p) => after(p) && squeeze(p.textContent ?? "") !== "")?.textContent ?? "");
  const kit = [...parsed.querySelectorAll("link")].some((link) => link.getAttribute("rel") === "stylesheet" && link.getAttribute("href") === KIT_HREF);
  const own = [...parsed.querySelectorAll("style")].map((style) => style.textContent ?? "").join("\n");
  const css = scopeCss(`${kit ? KIT_CSS : ""}\n${own}`);
  const sections = [...root.querySelectorAll("h2")].map((h2) => ({ id: h2.id, title: squeeze(h2.textContent ?? "") }));
  return { title, lede, sections, root, css, sketches: gathered.sketches };
}

/** The heading of the section an element is in: the last h2 before it, or the page's title above the first. */
export function sectionOf(root: ParentNode, element: Element, fallback: string): string {
  if (element.tagName === "H2") return squeeze(element.textContent ?? "");
  let found = fallback;
  for (const h2 of Array.from(root.querySelectorAll("h2"))) {
    if ((h2.compareDocumentPosition(element) & Node.DOCUMENT_POSITION_FOLLOWING) === 0) break;
    found = squeeze(h2.textContent ?? "");
  }
  return found;
}

/** The text of a row of a table, one cell after another, as a note names it. */
export const rowText = (row: Element): string => [...row.children].map((cell) => onOneLine(cell.textContent ?? "")).join(" | ");
