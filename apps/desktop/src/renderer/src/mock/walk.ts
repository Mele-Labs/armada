// A walk: a short named script of steps over a mock scenario, which the mock
// plays in the browser on `?walk=<name>`, `capture/walk.mjs` photographs, and
// the `walks-*of4.test.tsx` files run as tests. `docs/practices/running-locally.md`, *Walks*.
//
// **One way of finding a target and one way of pressing it, for all three.**
// A test that pressed with Playwright while the link pressed with this would
// pass on a walk the owner then found broken, and the walk is only evidence if
// what CI ran is what he clicks.

import { selectInEditor, typeIntoEditor } from "@armada/components";

import { PATIENCE_MS, WALK_UI, arriveAt, find } from "../walk-find";
import type { Name } from "../walk-find";
import { timePasses } from "./time-passes";

// The finding is shared with Bridge's capture window, which plays a walk on any page.
export { PATIENCE_MS, WALK_UI, find };
export type { Name };

/** Something on screen, found the way a test finds it: by role and accessible name, or by its text. */
export type Target = {
  /** The role, or `text` for an element found by the words it draws. */
  role: string;
  name?: Name;
  /** The whole name and nothing else, rather than a string inside it. */
  exact?: boolean;
  /** Looked for only inside this one. */
  within?: Target;
  /** What to look for instead where this is not on the page, in this order. */
  or?: Target[];
  /** How a stop names it. */
  said: string;
};

export type Step =
  | { press: Target; say: string }
  | { look: Target; say: string }
  /** Pointed at and held there while the step is shown, so what hovering reveals is in its picture. */
  | { hover: Target; say: string }
  /** Looked at, and then time passes: the scenario publishes its next moment as the walk moves on. */
  | { later: Target; say: string }
  /** Typed into a field; words ending in a newline end with Enter. */
  | { type: string; into: Target; say: string }
  /** A key pressed on a target as the keyboard sends one: "n", "Tab", or "Meta+n" for a chord. */
  | { key: string; on: Target; say: string }
  /** A right press on the target, as a pointer makes one: the context menu the app or the layer opens there. */
  | { context: Target; say: string }
  /** These words, inside the target, selected as a pointer drag would leave them. */
  | { select: string; in: Target; say: string }
  /** A screenshot pasted into a field, as a browser hands one over: a paste event carrying a PNG. */
  | { paste: Target; say: string }
  /** Words pasted on a target, as a browser hands them over: a paste event carrying plain text. */
  | { paste: Target; text: string; say: string }
  /** Picked up by its middle and put down `by` this far away, in screen pixels — a node on a canvas. `shift` holds Shift down through it. */
  | { drag: Target; by: { x: number; y: number }; shift?: true; say: string };

/** A walk is played at the window the test starts from unless it names its own: a narrow one shows what folds. */
export type Walk = { scenario: string; steps: readonly Step[]; viewport?: { width: number; height: number } };

/** A walk over `scenario`. Its name is the name it is exported under. */
export function walk(scenario: string, steps: readonly Step[], viewport?: Walk["viewport"]): Walk {
  return viewport === undefined ? { scenario, steps } : { scenario, steps, viewport };
}

const quoted = (name: Name) => (typeof name === "string" ? `“${name}”` : String(name));

/** Anything with this role, and this name where one is given. */
export function role(kind: string, name?: Name, options: { exact?: boolean } = {}): Target {
  return {
    role: kind,
    ...(name === undefined ? {} : { name }),
    ...(options.exact === undefined ? {} : { exact: options.exact }),
    said: name === undefined ? `a ${kind}` : `a ${kind} named ${quoted(name)}`,
  };
}

export const tab = (name: Name, options?: { exact?: boolean }) => role("tab", name, options);
export const button = (name: Name, options?: { exact?: boolean }) => role("button", name, options);
export const dialog = (name: Name) => role("dialog", name);
export const row = (name: Name) => role("row", name);
export const region = (name: Name, options?: { exact?: boolean }) => role("region", name, options);

/** Words on screen: the smallest element that draws them. */
export function text(words: Name): Target {
  return { role: "text", name: words, said: `the words ${quoted(words)}` };
}

/**
 * A card on a canvas: a Workflow step or a Plan group or task. **Named by its
 * label and then its state**, "Implement, running", so a label is matched up to
 * that comma rather than inside the name, where `Plan` would also find `Plan the change`.
 */
export function card(label: string): Target {
  const escaped = label.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  return { role: "button", name: new RegExp(`^${escaped}, `), said: `a card named “${label}”` };
}

/**
 * `first`, or else the next of `others` that is on the page: a place that is drawn two ways, as Back is a button where a panel opened over another and
 * Close is the one where it opened over the page.
 */
export function either(first: Target, ...others: Target[]): Target {
  return { ...first, or: others, said: [first, ...others].map((one) => one.said).join(" or ") };
}

/** `target`, looked for only inside `scope`. */
export function inside(scope: Target, target: Target): Target {
  return { ...target, within: scope, said: `${target.said} in ${scope.said}` };
}

/** One of the title bar's six places: Cockpit, Jobs, Sessions, Studios, Logbook or System. Not matched whole: a tab with a call waiting names it too. */
export const place = (name: string): Target => inside(role("navigation", "Places"), tab(name));

/** A row of the Cockpit's list of Sessions, by its title. */
export const deckRow = (title: Name): Target => inside(region("Jobs", { exact: true }), button(title));

/**
 * What the System strip's five pages were called, and the sidebar entry each opens now. A name that is an entry
 * of its own (Guides, Workflows, Limits, Output styles) is passed through.
 */
const ENTRY: Readonly<Record<string, string>> = { Kit: "Skills", Mods: "Theme", Settings: "Limits" };

/** One entry of the System sidebar, which is drawn only while System is the place. Matched on its name, so a mark beside it does not hide it. */
export const systemPage = (name: string): Target => {
  const entry = ENTRY[name] ?? name;
  return inside(role("navigation", "System"), button(new RegExp(`^${entry.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}( |$)`)));
};

/** The steps that reach a System entry from anywhere: System, then the entry in its sidebar. Studios is a place of its own, one press, and Retros is the Logbook's Retros tab. */
export function toSystemPage(name: string, say: string): Step[] {
  if (name === "Retros") return [{ press: place("Logbook"), say: "The Logbook" }, { press: tab("Retros"), say }];
  if (name === "Studios") return [{ press: place(name), say }];
  return [
    { press: place("System"), say: `System, for ${name}` },
    { press: systemPage(name), say },
  ];
}

/** The steps that reach a page by its name from ⌘K: the palette opened, the name typed, Enter. For a page no tab or System entry carries. */
export function viaPalette(name: string, say: string): Step[] {
  return [
    { key: "Meta+k", on: role("navigation", "Places"), say: `⌘K, for ${name}` },
    { type: `${name}\n`, into: inside(dialog("Command palette"), role("combobox")), say },
  ];
}

/** Where a step points: what it presses, looks at, or types into. */
export function targetOf(step: Step): Target {
  if ("hover" in step) return step.hover;
  if ("context" in step) return step.context;
  if ("key" in step) return step.on;
  if ("later" in step) return step.later;
  if ("paste" in step) return step.paste;
  if ("select" in step) return step.in;
  return "press" in step ? step.press : "look" in step ? step.look : "drag" in step ? step.drag : step.into;
}

/** What the step does, said plainly for a stop. */
function verb(step: Step): string {
  if ("hover" in step) return "hover over";
  if ("context" in step) return "right-click";
  if ("key" in step) return "press a key on";
  if ("later" in step) return "look at";
  if ("paste" in step) return "paste into";
  if ("select" in step) return "select words in";
  return "press" in step ? "press" : "look" in step ? "look at" : "drag" in step ? "drag" : "type into";
}

const frame = () => new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
const sleep = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms));

/**
 * The step's target once it is on screen and has stopped moving, or null if it
 * never came: `arriveAt`, which Bridge's capture window plays too. A hover is
 * then pointed at, so the step is shown with what the hover opens already open.
 */
export async function arrive(step: Step, patience = PATIENCE_MS): Promise<HTMLElement | null> {
  const found = await arriveAt(targetOf(step), patience);
  if (found !== null && "hover" in step) await pointAt(found);
  return found;
}

/** Why the walk stopped on a step, in the words its card and its test say. */
export function stopped(at: number, step: Step): string {
  return `Stopped at step ${at + 1}, “${step.say}”. Nothing to ${verb(step)}: no ${targetOf(step).said.replace(/^an? /, "")} came within ${PATIENCE_MS / 1000} seconds.`;
}

/**
 * A press as a pointer makes one: down, focus, up, click, at the element's
 * middle. **Not `element.click()`**, which skips the down a tab activates on.
 */
function press(element: HTMLElement): void {
  const box = element.getBoundingClientRect();
  const at = { bubbles: true, cancelable: true, composed: true, view: window, clientX: box.x + box.width / 2, clientY: box.y + box.height / 2, button: 0 };
  const pointer = { ...at, pointerId: 1, pointerType: "mouse", isPrimary: true };
  element.dispatchEvent(new PointerEvent("pointerdown", { ...pointer, buttons: 1 }));
  const down = element.dispatchEvent(new MouseEvent("mousedown", { ...at, buttons: 1 }));
  if (down) element.focus();
  element.dispatchEvent(new PointerEvent("pointerup", pointer));
  element.dispatchEvent(new MouseEvent("mouseup", at));
  element.dispatchEvent(new MouseEvent("click", at));
}

/** A right press as a pointer makes one: down, then the context menu event, at the element's middle. */
function pressContext(element: HTMLElement): void {
  const box = element.getBoundingClientRect();
  const at = { bubbles: true, cancelable: true, composed: true, view: window, clientX: box.x + box.width / 2, clientY: box.y + box.height / 2, button: 2 };
  element.dispatchEvent(new PointerEvent("pointerdown", { ...at, pointerId: 1, pointerType: "mouse", isPrimary: true, buttons: 2 }));
  element.dispatchEvent(new MouseEvent("mousedown", { ...at, buttons: 2 }));
  element.dispatchEvent(new MouseEvent("contextmenu", at));
}

/** A pointer arriving from outside the window, as React hears one: an over with nothing it came from. */
function pointerOver(element: HTMLElement, arriving: boolean): void {
  const box = element.getBoundingClientRect();
  const at = { bubbles: true, cancelable: true, composed: true, view: window, clientX: box.x + box.width / 2, clientY: box.y + box.height / 2, relatedTarget: null };
  const pointer = { ...at, pointerId: 1, pointerType: "mouse", isPrimary: true };
  element.dispatchEvent(new PointerEvent(arriving ? "pointerover" : "pointerout", pointer));
  element.dispatchEvent(new MouseEvent(arriving ? "mouseover" : "mouseout", at));
}

/**
 * The pointer put on the element and held there, **then the tooltip delay
 * waited out**, so the step is shown with what the hover opens already open.
 * The delay is read from its token, `--tooltip-delay`, as the tooltip reads it.
 */
async function pointAt(element: HTMLElement): Promise<void> {
  pointerOver(element, true);
  const delay = Number.parseInt(getComputedStyle(document.documentElement).getPropertyValue("--tooltip-delay"), 10);
  await sleep((Number.isNaN(delay) ? 400 : delay) + 100);
  await frame();
}

/** A field filled as typing fills it: React hears an input with the new value. */
function fill(element: HTMLElement, words: string): void {
  // A code editor draws in a shadow root, which a walk cannot reach into: it is typed at through its own view.
  if (typeIntoEditor(element, words)) return;
  // A select takes the option whose value is the words, and says so with
  // `change`, which is the event React reads a select's `onChange` from.
  if (element instanceof HTMLSelectElement) {
    element.focus();
    Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, "value")?.set?.call(element, words);
    element.dispatchEvent(new Event("change", { bubbles: true }));
    return;
  }
  // A box that holds tags in its lines is not a field: its words are its text, and it says so with `input`.
  if (element.isContentEditable) {
    element.focus();
    // A quote or a list standing in the box stays too, and the words go on the empty line under it, or on a new one.
    const standing = element.querySelector(":scope > [data-block]") !== null;
    let line: HTMLElement = element;
    if (standing) {
      const last = element.lastElementChild;
      line = last instanceof HTMLDivElement && last.dataset.block === undefined && last.textContent === "" ? last : element.appendChild(document.createElement("div"));
      line.replaceChildren();
    } else {
      // The words are put in, and the tags already standing in the line stay: they are not words.
      [...element.childNodes].filter((one) => !(one instanceof HTMLElement && one.dataset.tagKind !== undefined)).forEach((one) => one.remove());
    }
    line.append(document.createTextNode(words));
    const range = document.createRange();
    range.selectNodeContents(line);
    range.collapse(false);
    window.getSelection()?.removeAllRanges();
    window.getSelection()?.addRange(range);
    element.dispatchEvent(new Event("input", { bubbles: true }));
    return;
  }
  // xterm reads a typed character from an `input` event that carries it, as a browser sends one, and not from the field's value.
  if (element.classList.contains("xterm-helper-textarea")) {
    element.focus();
    for (const data of words) element.dispatchEvent(new InputEvent("input", { data, inputType: "insertText", bubbles: true }));
    return;
  }
  // Anything else is typed at as keys: each character goes to the element as a key press, for a page that listens for them.
  if (!(element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement)) {
    for (const key of words) element.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true }));
    return;
  }
  element.focus();
  const setter = Object.getOwnPropertyDescriptor(Object.getPrototypeOf(element), "value")?.set;
  setter?.call(element, words);
  element.dispatchEvent(new Event("input", { bubbles: true }));
}

/**
 * A drag as a mouse makes one: down on the element's middle, a few moves on
 * the page, and up where it ends. **On the document and not the element**,
 * which is where a canvas listens once a drag has started — React Flow's drag
 * follows the pointer off the node it picked up, and its connector listens on
 * the document. What is sent to the document reaches the window too.
 */
function drag(element: HTMLElement, by: { x: number; y: number }, shiftKey = false): void {
  const box = element.getBoundingClientRect();
  const from = { x: box.x + box.width / 2, y: box.y + box.height / 2 };
  const at = (x: number, y: number, buttons: number) => ({
    bubbles: true,
    cancelable: true,
    composed: true,
    view: window,
    clientX: x,
    clientY: y,
    button: 0,
    buttons,
    shiftKey,
  });
  // A resize handle listens for pointer events on itself, having captured the
  // pointer, so each step is sent there as a pointer event too.
  const pointer = (x: number, y: number, buttons: number) => ({ ...at(x, y, buttons), pointerId: 1, pointerType: "mouse", isPrimary: true });
  element.dispatchEvent(new PointerEvent("pointerdown", pointer(from.x, from.y, 1)));
  element.dispatchEvent(new MouseEvent("mousedown", at(from.x, from.y, 1)));
  for (const part of [0.25, 0.5, 0.75, 1]) {
    element.dispatchEvent(new PointerEvent("pointermove", pointer(from.x + by.x * part, from.y + by.y * part, 1)));
    document.dispatchEvent(new MouseEvent("mousemove", at(from.x + by.x * part, from.y + by.y * part, 1)));
  }
  element.dispatchEvent(new PointerEvent("pointerup", pointer(from.x + by.x, from.y + by.y, 0)));
  document.dispatchEvent(new MouseEvent("mouseup", at(from.x + by.x, from.y + by.y, 0)));
}

/**
 * A screenshot pasted as a browser delivers one: a paste event on the field
 * carrying a PNG in a clipboard of its own. **Never the machine's clipboard**,
 * so a walk, a test and a person's own copy cannot meet. The picture is drawn
 * here from the window's own tokens: a window with a failing test in it.
 */
function pasteScreenshot(element: HTMLElement): void {
  const tone = (name: string) => getComputedStyle(document.documentElement).getPropertyValue(name).trim() || "gray";
  const canvas = document.createElement("canvas");
  canvas.width = 480;
  canvas.height = 280;
  const draw = canvas.getContext("2d");
  if (draw === null) return;
  draw.fillStyle = tone("--bg-sunken");
  draw.fillRect(0, 0, 480, 280);
  draw.fillStyle = tone("--border-default");
  draw.fillRect(0, 0, 480, 28);
  draw.font = `${tone("--text-xs")} monospace`;
  const lines: [string, string][] = [
    ["--fg-muted", "running 31 tests"],
    ["--fg-muted", "test store_open ... ok"],
    ["--fg-muted", "test store_close ... ok"],
    ["--status-completed-failed", "test store_flaky ... FAILED"],
    ["--status-completed-failed", "assertion failed: at.second() == 0"],
  ];
  lines.forEach(([color, words], at) => {
    draw.fillStyle = tone(color);
    draw.fillText(words, 16, 64 + at * 24);
  });
  canvas.toBlob((blob) => {
    if (blob === null) return;
    const clipboard = new DataTransfer();
    clipboard.items.add(new File([blob], "Screenshot 2026-10-07 at 14.02.png", { type: "image/png" }));
    element.dispatchEvent(new ClipboardEvent("paste", { clipboardData: clipboard, bubbles: true, cancelable: true }));
  }, "image/png");
}

/**
 * Words pasted as a browser delivers them, in a clipboard of the walk's own: never the machine's. Where nothing took the
 * paste and the target is a field, the words go in after what was there, as a browser's own paste puts them.
 */
function pasteText(element: HTMLElement, text: string): void {
  const clipboard = new DataTransfer();
  clipboard.setData("text/plain", text);
  const went = element.dispatchEvent(new ClipboardEvent("paste", { clipboardData: clipboard, bubbles: true, cancelable: true }));
  if (went && (element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement)) fill(element, element.value + text);
}

/** `words` selected where they stand in `element`'s text, as the page hears a selection change. A code editor selects in its own view. */
function selectWords(element: HTMLElement, words: string): void {
  if (selectInEditor(element, words)) return;
  const nodes = document.createTreeWalker(element, NodeFilter.SHOW_TEXT);
  for (let node = nodes.nextNode(); node !== null; node = nodes.nextNode()) {
    const at = (node as Text).data.indexOf(words);
    if (at < 0) continue;
    const range = document.createRange();
    range.setStart(node, at);
    range.setEnd(node, at + words.length);
    window.getSelection()?.removeAllRanges();
    window.getSelection()?.addRange(range);
    return;
  }
}

/** A key as the keyboard sends it: `Alt+1` is the 1 key with Alt down, `Meta+n` a chord, and a key on its own is itself. */
function pressKey(element: HTMLElement, spec: string): void {
  const parts = spec.split("+");
  const key = parts[parts.length - 1] === "" ? "+" : parts[parts.length - 1]!;
  const held = new Set(parts.slice(0, -1));
  const code = /^[0-9]$/.test(key) ? `Digit${key}` : /^[a-z]$/i.test(key) ? `Key${key.toUpperCase()}` : key;
  // xterm reads a Control chord from the legacy key code, which is the letter's capital.
  const keyCode = /^[0-9a-z]$/i.test(key) ? key.toUpperCase().charCodeAt(0) : 0;
  element.dispatchEvent(
    new KeyboardEvent("keydown", { key, code, keyCode, altKey: held.has("Alt"), shiftKey: held.has("Shift"), metaKey: held.has("Meta"), ctrlKey: held.has("Control"), bubbles: true, cancelable: true }),
  );
}

/** The hover a press inside its card is waiting on, let go once that press is made. */
let held: HTMLElement | null = null;

/**
 * What the step does to its target when the walk moves past it. A look does nothing; a hover lets go, **unless the next
 * step presses**, as a pointer moves into the card it opened: a card closes a beat after its chip is let go, and a person
 * takes longer than that to press Next.
 */
export function act(step: Step, element: HTMLElement, next?: Step): void {
  const letGo = held;
  held = null;
  if ("hover" in step && next !== undefined && "press" in next) held = element;
  else if ("hover" in step) pointerOver(element, false);
  else if ("context" in step) pressContext(element);
  else if ("key" in step) pressKey(element, step.key);
  else if ("later" in step) timePasses();
  else if ("press" in step) press(element);
  else if ("drag" in step) drag(element, step.by, step.shift === true);
  else if ("type" in step) {
    // Words ending in a newline are sent with Enter, as a field that saves on it is used.
    fill(element, step.type.replace(/\n$/, ""));
    // A beat later, so the field has drawn what was typed before the key reaches it. xterm reads the legacy key code.
    if (step.type.endsWith("\n")) window.setTimeout(() => element.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", code: "Enter", keyCode: 13, bubbles: true })), 50);
  } else if ("paste" in step && "text" in step) pasteText(element, step.text);
  else if ("paste" in step) pasteScreenshot(element);
  else if ("select" in step) selectWords(element, step.select);
  if (letGo !== null) pointerOver(letGo, false);
}

/** Every step, in order, on the app already mounted. Throws on the first one whose target never came. */
export async function walkThrough(steps: readonly Step[]): Promise<void> {
  for (const [at, step] of steps.entries()) {
    const element = await arrive(step);
    if (element === null) throw new Error(stopped(at, step));
    act(step, element, steps[at + 1]);
    // The Enter a newline sends comes a beat after the words, and a person takes longer than that to press Next: let it land before the next step looks.
    if ("type" in step && step.type.endsWith("\n")) await sleep(120);
  }
}
