// The working mark turns at the display's rate, and the same loop on the map takes the clock. Browser: the real stylesheet, so a renamed
// rule or a moved region mark is caught here.

import { afterEach, beforeEach, expect, test } from "vitest";
import { commands } from "vitest/browser";

import { startLoopClock } from "./loop-clock";
import "./styles/index.css";

const wait = (ms: number) => new Promise((done) => window.setTimeout(done, ms));
let stop: () => void = () => undefined;

beforeEach(async () => {
  await commands.motion(true);
});

afterEach(async () => {
  await commands.motion(false);
  stop();
  document.body.replaceChildren();
});

/** A working mark's turning dot, in a fresh element appended to `host`. */
function mark(host: HTMLElement): SVGGElement {
  host.insertAdjacentHTML(
    "beforeend",
    '<span class="armada-working-mark"><svg width="14" height="14" viewBox="0 0 16 16"><g class="armada-working-mark__spin"><circle cx="8" cy="2" r="1.4" /></g></svg></span>',
  );
  return host.querySelector<SVGGElement>(".armada-working-mark__spin:last-of-type")!;
}

test("a working mark runs free, and the same mark on the map takes the clock", async () => {
  const free = mark(document.body);
  const map = document.createElement("div");
  map.setAttribute("data-loop-clock", "");
  document.body.append(map);
  const tamed = mark(map);
  stop = startLoopClock();
  await wait(300);

  const [turning] = free.getAnimations();
  const [stepped] = tamed.getAnimations();
  expect((turning as CSSAnimation).animationName).toBe("armada-working-spin");
  expect(turning!.playState).toBe("running");
  expect(stepped!.playState).toBe("paused");
});
