// Finding what a walk's step names on a page, and waiting for it to hold still. **One way of finding a
// target for every walk**: the mock plays it in its own page (`mock/walk.ts`), and Bridge's capture
// window plays it in an isolated world of another repository's page (`walk-find-page.ts`, built into a
// string by `electron.vite.config.ts`). A copy in either would find different elements than the
// other, and the walk is only evidence if what the test found is what the owner is shown.
//
// Nothing here writes to the page: it reads the tree, scrolls the target into view, and waits.

import { computeAccessibleName, getRole, isInaccessible } from "dom-accessibility-api";

/** What a name is matched against: a string inside it, any case, or a pattern. */
export type Name = string | RegExp;

/** Something on screen, found the way a test finds it: by role and accessible name, or by its text. */
export type Seeking = {
  /** The role, or `text` for an element found by the words it draws. */
  role: string;
  name?: Name;
  /** The whole name and nothing else, rather than a string inside it. */
  exact?: boolean;
  /** Looked for only inside this one. */
  within?: Seeking;
};

const squeezed = (words: string) => words.replace(/\s+/g, " ").trim();

function matches(name: string, wanted: Name, exact: boolean): boolean {
  const said = squeezed(name);
  if (typeof wanted !== "string") return wanted.test(said);
  return exact ? said === squeezed(wanted) : said.toLowerCase().includes(squeezed(wanted).toLowerCase());
}

/** The walk's own card and ring, which a target is never found in. */
export const WALK_UI = "data-walk-ui";

function reachable(element: Element): boolean {
  // The rows of a terminal's screen are drawn as text but hidden from the accessibility tree, which a person
  // reads them from, so a walk finds the words there as they are drawn.
  return element.closest(`[${WALK_UI}]`) === null && (!isInaccessible(element) || element.closest(".xterm-rows") !== null);
}

/** An element's role. A password field is a textbox, as Chromium and Playwright read it, where the library names none. */
const roleOf = (one: Element): string | null => getRole(one) ?? (one instanceof HTMLInputElement && one.type === "password" ? "textbox" : null);

/**
 * Every element under `root` this target names, in document order. **Hidden
 * ones are not found**, as a test's `getByRole` does not find them.
 */
function every(target: Seeking, root: ParentNode): Element[] {
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
      roleOf(one) === target.role &&
      reachable(one) &&
      (target.name === undefined || matches(computeAccessibleName(one), target.name, target.exact ?? false)),
  );
}

/**
 * The element a target names, or null. **The last of several**, as the mock's
 * tests take `.last()`: a panel is drawn after what it opens over, so the last
 * `Close` is the one on top.
 */
export function find(target: Seeking, root: ParentNode = document): HTMLElement | null {
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
 * The target once it is on screen and has stopped moving, or null if it
 * never came. **Still, and not only present**: a panel's first frame is drawn
 * wholly outside the window, and a press aimed at it then reaches nothing
 * (`docs/practices/bridge.md`, *A press waits for the surface to arrive*). So
 * it waits for the panel it sits in to finish entering, then for its box to
 * hold still across frames.
 */
export async function arriveAt(target: Seeking, patience = PATIENCE_MS): Promise<HTMLElement | null> {
  const until = Date.now() + patience;
  let found = find(target);
  while (found === null && Date.now() < until) {
    await sleep(100);
    found = find(target);
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
    return left <= 0 ? null : arriveAt(target, left);
  }
  // A surface that plays an arrival or a departure says so with `data-settles`: its picture is taken
  // once every animation that ends has ended, and one that loops is not waited for.
  const settling = found.closest("[data-settles]");
  if (settling !== null) {
    const ending = settling.getAnimations({ subtree: true }).filter((one) => one.effect?.getComputedTiming().iterations !== Infinity);
    await Promise.all(ending.map((one) => one.finished.catch(() => undefined)));
  }
  return found;
}
