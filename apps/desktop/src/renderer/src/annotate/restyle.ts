// A note's changes, put on its element so the person sees them, and taken off again when its card
// closes, so the app is never left restyled. Each property's own inline value is kept as it was found;
// Size and Colour are a rule of the note's own, which reaches inside the element as well.

import type { StyleChange } from "../../../shared/annotations";

type Kept = { styles: Map<string, { value: string; priority: string }>; text?: { node: Text; value: string }; id: string };

const kept = new WeakMap<Element, Kept>();

/** The element's first text with words in it, which the Text tool edits. Null where it draws none. */
export function wordsOf(element: Element): Text | null {
  const walker = document.createTreeWalker(element, NodeFilter.SHOW_TEXT, {
    acceptNode: (node) => ((node.nodeValue ?? "").trim() === "" ? NodeFilter.FILTER_SKIP : NodeFilter.FILTER_ACCEPT),
  });
  return walker.nextNode() as Text | null;
}

/** What `property` computes to on `element` now, as the page draws it. */
export function computed(element: Element, property: string): string {
  return getComputedStyle(element).getPropertyValue(property).trim();
}

/** A colour as the page computes it, `rgb(74, 158, 219)`, written as the tokens are, `#4A9EDB`; one with no alpha left is `transparent`. Anything else as it came. */
export function asHex(value: string): string {
  const parts = /^rgba?\(\s*(\d+)[,\s]+(\d+)[,\s]+(\d+)(?:[,\s/]+([\d.]+%?))?\s*\)$/.exec(value);
  if (parts === null) return value;
  const alpha = parts[4] === undefined ? 1 : parts[4].endsWith("%") ? Number(parts[4].slice(0, -1)) / 100 : Number(parts[4]);
  if (alpha === 0) return "transparent";
  const hex = [parts[1], parts[2], parts[3]].map((part) => Number(part).toString(16).padStart(2, "0").toUpperCase()).join("");
  return alpha === 1 ? `#${hex}` : `#${hex}${Math.round(alpha * 255).toString(16).padStart(2, "0").toUpperCase()}`;
}

/** What a token resolves to where `element` sits, so a theme's own value is the one recorded. */
export function tokenValue(element: Element, token: string): string {
  return getComputedStyle(element).getPropertyValue(token).trim();
}

/** The attribute that marks a note's element, so its rule reaches the element and everything inside it. */
export const RESTYLED = "data-armada-restyled";

/** Size and Colour reach what is inside the element too: "larger" is about what is in the area he picked. */
const REACHES_INSIDE: ReadonlySet<string> = new Set(["font-size", "color"]);

/** Each note's rule, by its id, written together into the one style element the layer owns. */
const rules = new Map<string, string>();
let sheet: HTMLStyleElement | null = null;

function writeRules(): void {
  if (sheet === null || !sheet.isConnected) {
    sheet = document.createElement("style");
    sheet.setAttribute("data-armada-restyle", "");
    document.head.append(sheet);
  }
  sheet.textContent = [...rules.values()].join("\n");
}

const valueOf = (change: StyleChange): string => (change.token === undefined ? change.after : `var(${change.token})`);

/**
 * Takes off whatever was put on before, then puts note `id`'s `changes` on: a token as the token, so it follows the theme.
 * Size and Colour go in one rule for the element and all inside it; Fill and Spacing inline on the element; Text in its words.
 */
export function putOn(element: Element, changes: readonly StyleChange[], id: string): void {
  takeOff(element);
  if (!(element instanceof HTMLElement) && !(element instanceof SVGElement)) return;
  const was: Kept = { styles: new Map(), id };
  const reaching: string[] = [];
  for (const change of changes) {
    if (change.kind === "text") {
      const node = was.text?.node ?? wordsOf(element);
      if (node === null) continue;
      was.text ??= { node, value: node.nodeValue ?? "" };
      node.nodeValue = change.after;
      continue;
    }
    if (REACHES_INSIDE.has(change.property)) {
      reaching.push(`${change.property}: ${valueOf(change)} !important;`);
      continue;
    }
    if (!was.styles.has(change.property)) {
      was.styles.set(change.property, { value: element.style.getPropertyValue(change.property), priority: element.style.getPropertyPriority(change.property) });
    }
    element.style.setProperty(change.property, valueOf(change), "important");
  }
  if (reaching.length > 0) {
    element.setAttribute(RESTYLED, id);
    const at = `[${RESTYLED}="${id}"]`;
    rules.set(id, `${at}, ${at} * { ${reaching.join(" ")} }`);
    writeRules();
  }
  kept.set(element, was);
}

/** Puts back the element's own inline values and words, as they were before `putOn`, and drops the note's rule. */
export function takeOff(element: Element): void {
  const was = kept.get(element);
  if (was === undefined) return;
  kept.delete(element);
  if (element instanceof HTMLElement || element instanceof SVGElement) {
    for (const [property, { value, priority }] of was.styles) {
      if (value === "") element.style.removeProperty(property);
      else element.style.setProperty(property, value, priority);
    }
  }
  if (was.text !== undefined) was.text.node.nodeValue = was.text.value;
  if (element.getAttribute(RESTYLED) === was.id) element.removeAttribute(RESTYLED);
  if (rules.delete(was.id)) writeRules();
}
