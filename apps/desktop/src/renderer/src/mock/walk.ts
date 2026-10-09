// A walk: a short named script of steps over a mock scenario, which the mock
// plays in the browser on `?walk=<name>`, `capture/walk.mjs` photographs, and
// the `walks-*of4.test.tsx` files run as tests. `docs/practices/running-locally.md`, *Walks*.
//
// **One way of finding a target and one way of pressing it, for all three.**
// A test that pressed with Playwright while the link pressed with this would
// pass on a walk the owner then found broken, and the walk is only evidence if
// what CI ran is what he clicks.

import { computeAccessibleName, getRole, isInaccessible } from "dom-accessibility-api";

import { timePasses } from "./time-passes";

/** What a name is matched against: a string inside it, any case, or a pattern. */
export type Name = string | RegExp;

/** Something on screen, found the way a test finds it: by role and accessible name, or by its text. */
export type Target = {
  /** The role, or `text` for an element found by the words it draws. */
  role: string;
  name?: Name;
  /** The whole name and nothing else, rather than a string inside it. */
  exact?: boolean;
  /** Looked for only inside this one. */
  within?: Target;
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
  /** A screenshot pasted into a field, as a browser hands one over: a paste event carrying a PNG. */
  | { paste: Target; say: string }
  /** A key pressed with nothing typed into: `1`, `Enter`, `Escape`, `Alt+1`. It reaches the window through `on`, which is waited for. */
  | { key: string; on: Target; say: string }
  /** Picked up by its middle and put down `by` this far away, in screen pixels — a node on a canvas. */
  | { drag: Target; by: { x: number; y: number }; say: string };

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

export const tab = (name: Name) => role("tab", name);
export const button = (name: Name, options?: { exact?: boolean }) => role("button", name, options);
export const dialog = (name: Name) => role("dialog", name);
export const row = (name: Name) => role("row", name);
export const region = (name: Name) => role("region", name);

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

/** `target`, looked for only inside `scope`. */
export function inside(scope: Target, target: Target): Target {
  return { ...target, within: scope, said: `${target.said} in ${scope.said}` };
}

/** Where a step points: what it presses, looks at, or types into. */
export function targetOf(step: Step): Target {
  if ("hover" in step) return step.hover;
  if ("later" in step) return step.later;
  if ("paste" in step) return step.paste;
  if ("key" in step) return step.on;
  return "press" in step ? step.press : "look" in step ? step.look : "drag" in step ? step.drag : step.into;
}

/** What the step does, said plainly for a stop. */
function verb(step: Step): string {
  if ("hover" in step) return "hover over";
  if ("later" in step) return "look at";
  if ("paste" in step) return "paste into";
  if ("key" in step) return "press a key at";
  return "press" in step ? "press" : "look" in step ? "look at" : "drag" in step ? "drag" : "type into";
}

const squeezed = (words: string) => words.replace(/\s+/g, " ").trim();

function matches(name: string, wanted: Name, exact: boolean): boolean {
  const said = squeezed(name);
  if (typeof wanted !== "string") return wanted.test(said);
  return exact ? said === squeezed(wanted) : said.toLowerCase().includes(squeezed(wanted).toLowerCase());
}

/** The walk's own card and ring, which a target is never found in. */
export const WALK_UI = "data-walk-ui";

function reachable(element: Element): boolean {
  return element.closest(`[${WALK_UI}]`) === null && !isInaccessible(element);
}

/**
 * Every element under `root` this target names, in document order. **Hidden
 * ones are not found**, as a test's `getByRole` does not find them.
 */
function every(target: Target, root: ParentNode): Element[] {
  const all = [...root.querySelectorAll("*")];
  if (target.role === "text") {
    const wanted = target.name!;
    const drawing = all.filter(
      (one) => !["SCRIPT", "STYLE"].includes(one.tagName) && matches(one.textContent ?? "", wanted, false),
    );
    // The smallest: one whose children do not draw the words on their own.
    return drawing.filter((one) => !drawing.some((other) => other !== one && one.contains(other))).filter(reachable);
  }
  return all.filter(
    (one) =>
      getRole(one) === target.role &&
      reachable(one) &&
      (target.name === undefined || matches(computeAccessibleName(one), target.name, target.exact ?? false)),
  );
}

/**
 * The element a target names, or null. **The last of several**, as the mock's
 * tests take `.last()`: a panel is drawn after what it opens over, so the last
 * `Close` is the one on top.
 */
export function find(target: Target, root: ParentNode = document): HTMLElement | null {
  let scope: ParentNode = root;
  if (target.within !== undefined) {
    const found = find(target.within, root);
    if (found === null) return null;
    scope = found;
  }
  const all = every(target, scope);
  return (all[all.length - 1] as HTMLElement | undefined) ?? null;
}

/** How long a step waits for its target before the walk stops on it. */
export const PATIENCE_MS = 5_000;

const frame = () => new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
const sleep = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms));

/**
 * The step's target once it is on screen and has stopped moving, or null if it
 * never came. **Still, and not only present**: a panel's first frame is drawn
 * wholly outside the window, and a press aimed at it then reaches nothing
 * (`docs/practices/bridge.md`, *A press waits for the surface to arrive*). So
 * it waits for the panel it sits in to finish entering, then for its box to
 * hold still across frames.
 */
export async function arrive(step: Step, patience = PATIENCE_MS): Promise<HTMLElement | null> {
  const until = Date.now() + patience;
  let found = find(targetOf(step));
  while (found === null && Date.now() < until) {
    await sleep(100);
    found = find(targetOf(step));
  }
  if (found === null) return null;
  found.scrollIntoView({ block: "nearest", inline: "nearest" });
  const layer = found.closest('[role="dialog"]');
  if (layer !== null) await Promise.all(layer.getAnimations().map((one) => one.finished.catch(() => undefined)));
  let was = found.getBoundingClientRect();
  for (let held = 0, tries = 0; held < 3 && tries < 60; tries += 1) {
    await frame();
    const now = found.getBoundingClientRect();
    held = now.x === was.x && now.y === was.y && now.width === was.width ? held + 1 : 0;
    was = now;
  }
  // **Gone while it was being waited on**: the surface it was found in was
  // replaced — a tab the last press switched — and one of the same name may
  // be on the one that replaced it. A press on the old one reaches nothing,
  // so look again, in what patience is left.
  if (!found.isConnected) {
    const left = until - Date.now();
    return left <= 0 ? null : arrive(step, left);
  }
  // A surface that plays an arrival or a departure says so with `data-settles`: its picture is taken
  // once every animation that ends has ended, and one that loops is not waited for.
  const settling = found.closest("[data-settles]");
  if (settling !== null) {
    const ending = settling.getAnimations({ subtree: true }).filter((one) => one.effect?.getComputedTiming().iterations !== Infinity);
    await Promise.all(ending.map((one) => one.finished.catch(() => undefined)));
  }
  if ("hover" in step) await pointAt(found);
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
    // The words are put in, and the tags already standing in the line stay: they are not words.
    [...element.childNodes].filter((one) => !(one instanceof HTMLElement && one.dataset.tagKind !== undefined)).forEach((one) => one.remove());
    element.append(document.createTextNode(words));
    const range = document.createRange();
    range.selectNodeContents(element);
    range.collapse(false);
    window.getSelection()?.removeAllRanges();
    window.getSelection()?.addRange(range);
    element.dispatchEvent(new Event("input", { bubbles: true }));
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
 * the window, and up where it ends. **On the window and not the element**,
 * which is where a canvas listens once a drag has started — React Flow's drag
 * follows the pointer off the node it picked up.
 */
function drag(element: HTMLElement, by: { x: number; y: number }): void {
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
  });
  // A resize handle listens for pointer events on itself, having captured the
  // pointer, so each step is sent there as a pointer event too.
  const pointer = (x: number, y: number, buttons: number) => ({ ...at(x, y, buttons), pointerId: 1, pointerType: "mouse", isPrimary: true });
  element.dispatchEvent(new PointerEvent("pointerdown", pointer(from.x, from.y, 1)));
  element.dispatchEvent(new MouseEvent("mousedown", at(from.x, from.y, 1)));
  for (const part of [0.25, 0.5, 0.75, 1]) {
    element.dispatchEvent(new PointerEvent("pointermove", pointer(from.x + by.x * part, from.y + by.y * part, 1)));
    window.dispatchEvent(new MouseEvent("mousemove", at(from.x + by.x * part, from.y + by.y * part, 1)));
  }
  element.dispatchEvent(new PointerEvent("pointerup", pointer(from.x + by.x, from.y + by.y, 0)));
  window.dispatchEvent(new MouseEvent("mouseup", at(from.x + by.x, from.y + by.y, 0)));
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

/** A key as the keyboard sends it: `Alt+1` is the 1 key with Alt down, and a key on its own is itself. */
function pressKey(element: HTMLElement, spec: string): void {
  const parts = spec.split("+");
  const key = parts[parts.length - 1] === "" ? "+" : parts[parts.length - 1]!;
  const held = new Set(parts.slice(0, -1));
  const code = /^[0-9]$/.test(key) ? `Digit${key}` : /^[a-z]$/i.test(key) ? `Key${key.toUpperCase()}` : key;
  element.dispatchEvent(
    new KeyboardEvent("keydown", { key, code, altKey: held.has("Alt"), shiftKey: held.has("Shift"), metaKey: held.has("Meta"), ctrlKey: held.has("Control"), bubbles: true, cancelable: true }),
  );
}

/** What the step does to its target when the walk moves past it. A look does nothing; a hover lets go. */
export function act(step: Step, element: HTMLElement): void {
  if ("hover" in step) pointerOver(element, false);
  else if ("later" in step) timePasses();
  else if ("press" in step) press(element);
  else if ("drag" in step) drag(element, step.by);
  else if ("type" in step) {
    // Words ending in a newline are sent with Enter, as a field that saves on it is used.
    fill(element, step.type.replace(/\n$/, ""));
    // A beat later, so the field has drawn what was typed before the key reaches it.
    if (step.type.endsWith("\n")) window.setTimeout(() => element.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true })), 50);
  } else if ("paste" in step) pasteScreenshot(element);
  else if ("key" in step) pressKey(element, step.key);
}

/** Every step, in order, on the app already mounted. Throws on the first one whose target never came. */
export async function walkThrough(steps: readonly Step[]): Promise<void> {
  for (const [at, step] of steps.entries()) {
    const element = await arrive(step);
    if (element === null) throw new Error(stopped(at, step));
    act(step, element);
  }
}
