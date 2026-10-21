// What the walk in a page tells the window that frames it, on `window.__armadaWalk`: a step whose target never came is
// stopped, and so is a walk the page has no file for, so main tells the Job rather than reading either as still looking.

import { afterEach, expect, onTestFinished, test } from "vitest";

import { mountNoWalk, mountWalk } from "./WalkPlayer";
import { PATIENCE_MS, button, text, walk } from "./walk";

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

test("a hover is held while the next step presses inside the card it opened, however long Next takes", async () => {
  const at = mounted();
  document.body.innerHTML = '<button id="chip">Chip</button>';
  document.body.appendChild(at);
  const chip = document.getElementById("chip")!;
  let card: HTMLElement | null = null;
  chip.addEventListener("mouseover", () => {
    if (card !== null) return;
    card = document.body.appendChild(document.createElement("div"));
    card.innerHTML = '<button data-open="">Open</button>';
  });
  // Heard at the document, as React hears a press: a press on a button gone from the page never reaches it.
  const opens = (event: Event) => {
    if ((event.target as HTMLElement).matches("[data-open]")) document.body.appendChild(document.createElement("p")).textContent = "Opened";
  };
  document.addEventListener("click", opens);
  onTestFinished(() => document.removeEventListener("click", opens));
  // A hover card closes a beat after the pointer leaves, as `Inspect`'s does.
  chip.addEventListener("mouseout", () => setTimeout(() => (card?.remove(), (card = null)), 150));
  mountWalk("probe", walk("none", [{ hover: button("Chip"), say: "Hover" }, { press: button("Open"), say: "Press inside its card" }, { look: text("Opened"), say: "Opened" }]), false, at);
  await expect.poll(() => window.__armadaWalk?.state().ready).toBe(true);
  window.__armadaWalk!.next();
  await expect.poll(() => window.__armadaWalk?.state()).toMatchObject({ at: 2, ready: true });
  // A person reads the step before pressing Next.
  await new Promise((resolve) => setTimeout(resolve, 400));
  window.__armadaWalk!.next();
  await expect.poll(() => window.__armadaWalk?.state()).toMatchObject({ at: 3, ready: true, stopped: false });
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
