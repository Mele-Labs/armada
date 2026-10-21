// What the walk in a page tells the window that frames it, on `window.__armadaWalk`: a step whose target never came is
// stopped, and so is a walk the page has no file for, so main tells the Job rather than reading either as still looking.

import { afterEach, expect, test } from "vitest";

import { mountNoWalk, mountWalk } from "./WalkPlayer";
import { PATIENCE_MS, text, walk } from "./walk";

let host: HTMLElement | undefined;
afterEach(() => {
  host?.remove();
  delete window.__armadaWalk;
});

function mounted(): HTMLElement {
  document.body.innerHTML = '<p id="target">Hello</p>';
  host = document.body.appendChild(document.createElement("div"));
  return host;
}

test("a step whose target never came is stopped on the handle, and one that came is not", async () => {
  mountWalk("probe", walk("none", [{ look: text("Hello"), say: "Look here" }, { look: text("Nowhere"), say: "Then here" }]), false, mounted());
  await expect.poll(() => window.__armadaWalk?.state().ready).toBe(true);
  expect(window.__armadaWalk!.state().stopped).toBe(false);
  window.__armadaWalk!.next();
  await expect.poll(() => window.__armadaWalk?.state().stopped, { timeout: PATIENCE_MS + 2_000 }).toBe(true);
  expect(window.__armadaWalk!.state()).toMatchObject({ at: 2, of: 2, ready: false, done: false });
}, PATIENCE_MS + 5_000);

test("a walk the page has no file for is a stopped walk on the handle, and its card leaves to the bar", async () => {
  mountNoWalk("nowhere", mounted());
  await expect.poll(() => window.__armadaWalk?.state()).toEqual({ name: "nowhere", at: 1, of: 1, said: "No walk named nowhere.", autoplay: false, ready: false, done: false, stopped: true });
  expect(document.querySelector("[data-walk-state]")).not.toBeNull();
  window.__armadaWalk!.hostCard(true);
  await expect.poll(() => document.querySelector("[data-walk-state]")).toBeNull();
  // Its acts do nothing: there is no step to move to.
  window.__armadaWalk!.next();
  window.__armadaWalk!.replay();
  expect(window.__armadaWalk!.state().stopped).toBe(true);
});
