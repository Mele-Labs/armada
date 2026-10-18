// What a note records about the element it was left on, read from the page.

import { annotationId, SOURCE_ATTRIBUTE, type Annotation, type Box, type Source } from "../../../shared/annotations";
import { componentsOf, fiberOf } from "./fiber";
import { selectorFor } from "./selector";

const TEXT = 80;

/** Visible text, collapsed and cut, so a note names what a person read on screen. */
export function shortText(text: string | null | undefined): string {
  const collapsed = (text ?? "").replace(/\s+/g, " ").trim();
  return collapsed.length > TEXT ? `${collapsed.slice(0, TEXT - 1)}…` : collapsed;
}

/**
 * The accessible name of the place the window marks as current: the System strip's page where one is drawn, else the title
 * bar's tab, each the selected tab of its own navigation. An element that sets `aria-current="page"` is read first.
 */
export function currentScreen(doc: Document): string | null {
  const current =
    doc.querySelector('[aria-current="page"]') ??
    doc.querySelector('nav[aria-label="System"] [role="tab"][aria-selected="true"]') ??
    doc.querySelector('nav[aria-label="Places"] [role="tab"][aria-selected="true"]');
  if (current === null) return null;
  const label = current.getAttribute("aria-label");
  if (label !== null) return label;
  // Beside the label the item draws a count and a binding, and neither is the screen's name.
  const named = current.querySelector(".armada-sidebar__label");
  if (named !== null) return shortText(named.textContent) || null;
  const copy = current.cloneNode(true) as Element;
  copy.querySelectorAll("kbd").forEach((kbd) => kbd.remove());
  return shortText(copy.textContent) || null;
}

/** The dialog or sheet the element is inside, by its accessible name. */
function layerOf(element: Element): string | null {
  const dialog = element.closest('[role="dialog"], [role="alertdialog"], dialog');
  if (dialog === null) return null;
  const labelledBy = dialog.getAttribute("aria-labelledby");
  const named = labelledBy === null ? null : element.ownerDocument.getElementById(labelledBy);
  return dialog.getAttribute("aria-label") ?? (named === null ? null : shortText(named.textContent)) ?? "dialog";
}

/**
 * Where the element's JSX is, off the nearest ancestor the build stamped
 * (`annotations-source.ts`). Undefined where nothing above it carries one —
 * which is every element under `pnpm dev` — because a note that names the
 * wrong file is worse than one that names none.
 */
export function sourceOf(element: Element): Source | undefined {
  const value = element.closest(`[${SOURCE_ATTRIBUTE}]`)?.getAttribute(SOURCE_ATTRIBUTE) ?? null;
  if (value === null) return undefined;
  const cut = value.lastIndexOf(":");
  const line = Number(value.slice(cut + 1));
  if (cut <= 0 || !Number.isInteger(line) || line < 1) return undefined;
  return { file: value.slice(0, cut), line };
}

// The capture window reads the same three in a foreign page, `main/capture/layer.ts`, and cannot import these.
const IMPLICIT: Record<string, string> = {
  a: "link", article: "article", aside: "complementary", button: "button", details: "group", dialog: "dialog",
  footer: "contentinfo", form: "form", header: "banner", li: "listitem", main: "main", menu: "list", nav: "navigation",
  ol: "list", option: "option", search: "search", section: "region", select: "combobox", summary: "button",
  table: "table", td: "cell", th: "columnheader", tr: "row", ul: "list",
};
const ENCLOSING = new Set([
  "banner", "navigation", "main", "complementary", "contentinfo", "region", "form", "search", "article",
  "list", "listbox", "menu", "menubar", "tablist", "tree", "grid", "table", "dialog", "alertdialog",
]);
const HEADINGS = 'h1, h2, h3, h4, h5, h6, [role="heading"]';
const MARKED =
  '[aria-selected="true"], [aria-current], [aria-pressed="true"], [aria-checked="true"], [aria-expanded="true"], dialog[open], details[open]';

function roleOf(node: Element): string {
  const explicit = (node.getAttribute("role") ?? "").trim().split(/\s+/)[0] ?? "";
  if (explicit !== "") return explicit;
  const tag = node.tagName.toLowerCase();
  if (/^h[1-6]$/.test(tag)) return "heading";
  if (tag === "input") {
    const type = (node.getAttribute("type") ?? "").toLowerCase();
    return type === "checkbox" || type === "radio" ? type : "textbox";
  }
  return IMPLICIT[tag] ?? tag;
}

/** `aria-label`, else what `aria-labelledby` points at: a container's name is never its text. */
function labelOf(node: Element): string {
  const label = shortText(node.getAttribute("aria-label"));
  if (label !== "") return label;
  const by = node.getAttribute("aria-labelledby");
  if (by === null) return "";
  return shortText(by.split(/\s+/).map((id) => (id === "" ? "" : (node.ownerDocument.getElementById(id)?.textContent ?? ""))).join(" "));
}

/** A heading's or a marked control's name: its label, else a `details`' summary, else its text. */
function nameOf(node: Element): string {
  const label = labelOf(node);
  if (label !== "") return label;
  const summary = node.tagName.toLowerCase() === "details" ? [...node.children].find((one) => one.tagName.toLowerCase() === "summary") : undefined;
  return shortText((summary ?? node).textContent);
}

/** The named landmarks, lists, sections and dialogs around the element, and the nearest heading above it, innermost first. At most 8. */
export function nearbyOf(element: Element): string[] {
  const names: string[] = [];
  const add = (name: string): void => {
    if (name !== "" && names[names.length - 1] !== name) names.push(name);
  };
  let headed = false;
  let at = element;
  while (names.length < 8) {
    for (let before = at.previousElementSibling; before !== null && !headed; before = before.previousElementSibling) {
      const inside = [...before.querySelectorAll(HEADINGS)].filter((one) => roleOf(one) === "heading");
      const hit = roleOf(before) === "heading" ? before : inside[inside.length - 1];
      if (hit !== undefined) {
        headed = true;
        add(nameOf(hit));
      }
    }
    const parent = at.parentElement;
    if (parent === null || parent.tagName === "BODY" || parent.tagName === "HTML") break;
    if (ENCLOSING.has(roleOf(parent))) add(labelOf(parent));
    at = parent;
  }
  return names.slice(0, 8);
}

/** What the window marks selected, current, open or pressed, each `role "name"`, the layer's own controls left out. At most 12. */
export function selectedIn(doc: Document): string[] {
  const said: string[] = [];
  for (const one of doc.querySelectorAll(MARKED)) {
    if (said.length >= 12) break;
    if (one.closest("[data-armada-annotate], [data-walk-ui]") !== null) continue;
    const current = one.getAttribute("aria-current");
    const tag = one.tagName.toLowerCase();
    const marked =
      (current !== null && current !== "" && current !== "false") ||
      ["aria-selected", "aria-pressed", "aria-checked", "aria-expanded"].some((name) => one.getAttribute(name) === "true") ||
      ((tag === "dialog" || tag === "details") && one.hasAttribute("open"));
    if (!marked) continue;
    const name = nameOf(one);
    const line = name === "" ? roleOf(one) : `${roleOf(one)} "${name}"`;
    if (!said.includes(line)) said.push(line);
  }
  return said;
}

/** The short selector when it finds this element first, and the whole chain when it does not. */
function findableSelector(element: Element): string {
  const selector = selectorFor(element);
  try {
    if (element.ownerDocument.querySelector(selector) === element) return selector;
  } catch {
    // An attribute value the engine would not parse; the whole chain is the better record.
  }
  return selectorFor(element, Number.POSITIVE_INFINITY);
}

/** A new, open note on `element`, with nothing said yet. */
export function capture(element: Element, now: Date, win: Window = window): Annotation {
  const rect = element.getBoundingClientRect();
  const { component, owners, from } = componentsOf(fiberOf(element));
  const source = sourceOf(element);
  const at = now.toISOString();
  return {
    id: annotationId(now),
    status: "open",
    text: "",
    component,
    ...(source === undefined ? {} : { source }),
    owners,
    ownersFrom: from,
    selector: findableSelector(element),
    element: {
      tag: element.tagName.toLowerCase(),
      text: shortText(element.textContent),
      label: element.getAttribute("aria-label"),
    },
    screen: currentScreen(element.ownerDocument),
    layer: layerOf(element),
    nearby: nearbyOf(element),
    location: `${win.location.pathname}${win.location.search}${win.location.hash}`,
    scenario: new URLSearchParams(win.location.search).get("scenario"),
    selected: selectedIn(element.ownerDocument),
    box: {
      x: Math.round(rect.x),
      y: Math.round(rect.y),
      width: Math.round(rect.width),
      height: Math.round(rect.height),
    },
    window: { width: win.innerWidth, height: win.innerHeight },
    createdAt: at,
    updatedAt: at,
  };
}

/** The window's own box, which a whole-window note is placed against and pictured by. */
export const windowBox = (win: Window = window): Box => ({ x: 0, y: 0, width: win.innerWidth, height: win.innerHeight });

/**
 * A new, open note on the whole window: the page's body stands in for an element, so the note
 * names the screen and the dialog that was up rather than one thing on it.
 */
export function captureWindow(now: Date, win: Window = window): Annotation {
  const doc = win.document;
  // The layer's own card is a dialog too, and never the app's.
  const top = [...doc.querySelectorAll('[role="dialog"], [role="alertdialog"], dialog[open]')].find((one) => one.closest("[data-armada-annotate]") === null) ?? null;
  return {
    ...capture(doc.body, now, win),
    scope: "window",
    component: null,
    owners: [],
    selector: "body",
    element: { tag: "body", text: "", label: null },
    layer: top === null ? null : layerOf(top),
    box: windowBox(win),
  };
}

/** The element a saved note points at, if the page still has one. */
export function locate(note: Annotation, doc: Document = document): Element | null {
  try {
    return doc.querySelector(note.selector);
  } catch {
    return null;
  }
}
