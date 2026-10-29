// What a Job's agent reads of its Job browser's page: the accessibility tree as text, one line a part, as a test finds them —
// role, name, value and state — so the target it names next is found by the same finder (`../walk-find.ts`). **Built into the
// walk finder** (`../walk-find-page.ts`) and run in an isolated world of the page, which shares its DOM and nothing else.

import { computeAccessibleName, getRole, isInaccessible } from "dom-accessibility-api";

/** Roles that say nothing of their own: their children are read in their place. */
const QUIET = new Set(["generic", "none", "presentation", "paragraph", "group", "list", "listitem", "row", "rowgroup"]);
/** Roles whose name is their text, so the text is not said twice. */
const NAMED_BY_TEXT = new Set(["button", "link", "heading", "tab", "menuitem", "menuitemcheckbox", "menuitemradio", "option", "checkbox", "radio", "switch", "treeitem", "cell", "columnheader", "rowheader", "label"]);
const VALUED = new Set(["textbox", "searchbox", "combobox", "spinbutton", "slider"]);
/** Elements past this are not read: a page that large is said to be cut. */
const MOST_ELEMENTS = 5_000;

const squeezed = (words: string): string => words.replace(/\s+/g, " ").trim();
const quoted = (words: string): string => JSON.stringify(words.length > 200 ? `${words.slice(0, 200)}…` : words);

function stateOf(element: Element, role: string): string[] {
  const said: string[] = [];
  const aria = (name: string) => element.getAttribute(`aria-${name}`);
  if (role === "heading") {
    const level = aria("level") ?? /^H([1-6])$/.exec(element.tagName)?.[1];
    if (level !== undefined && level !== null) said.push(`level=${level}`);
  }
  const checked = element instanceof HTMLInputElement && (element.type === "checkbox" || element.type === "radio") ? element.checked : aria("checked") === "true";
  if (checked) said.push("checked");
  if ((element as HTMLInputElement).disabled === true || aria("disabled") === "true") said.push("disabled");
  for (const flag of ["expanded", "selected", "pressed"]) if (aria(flag) === "true") said.push(flag);
  if (element instanceof HTMLOptionElement && element.selected) said.push("selected");
  return said;
}

function valueOf(element: Element): string | undefined {
  if (element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement) return element.type === "password" ? (element.value === "" ? "" : "••••") : element.value;
  if (element instanceof HTMLSelectElement) return element.selectedOptions[0]?.textContent ?? "";
  return element.getAttribute("aria-valuetext") ?? element.getAttribute("aria-valuenow") ?? undefined;
}

/** The page under `root` as an outline, cut at `most` characters with a last line that says so. */
export function snapshotOf(root: Element, most: number): string {
  const lines: string[] = [];
  let length = 0;
  let read = 0;
  let cut = false;
  const say = (depth: number, line: string): void => {
    if (cut) return;
    const text = `${"  ".repeat(depth)}- ${line}`;
    if (length + text.length + 1 > most) {
      cut = true;
      return;
    }
    lines.push(text);
    length += text.length + 1;
  };
  const walk = (node: Node, depth: number, quiet: boolean): void => {
    if (cut) return;
    if (node.nodeType === Node.TEXT_NODE) {
      const words = squeezed(node.textContent ?? "");
      if (words !== "" && !quiet) say(depth, `text: ${quoted(words)}`);
      return;
    }
    if (!(node instanceof Element) || ["SCRIPT", "STYLE", "NOSCRIPT", "TEMPLATE"].includes(node.tagName)) return;
    if ((read += 1) > MOST_ELEMENTS) {
      cut = true;
      return;
    }
    if (isInaccessible(node)) return;
    const role = getRole(node) ?? (node instanceof HTMLInputElement && node.type === "password" ? "textbox" : null);
    let inner = depth;
    let hushed = quiet;
    if (role !== null && !QUIET.has(role)) {
      const name = squeezed(computeAccessibleName(node));
      const value = VALUED.has(role) ? valueOf(node) : undefined;
      const state = stateOf(node, role);
      say(depth, `${role}${name === "" ? "" : ` ${quoted(name)}`}${state.length === 0 ? "" : ` [${state.join(", ")}]`}${value === undefined ? "" : `: ${quoted(value)}`}`);
      inner = depth + 1;
      hushed = quiet || NAMED_BY_TEXT.has(role) || VALUED.has(role);
    }
    for (const child of node.childNodes) walk(child, inner, hushed);
  };
  walk(root, 0, false);
  if (cut) lines.push("- (cut: the page goes on)");
  return lines.join("\n");
}
