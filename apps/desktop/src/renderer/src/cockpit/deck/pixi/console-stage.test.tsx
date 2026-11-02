// When the console draws. **Its lamps move on the loop clock, with no ticker of its own**, because any frame redraws the whole window;
// an idle window holds them, and reduced motion draws what its screens read once a second. Browser: the real stage on a real WebGL canvas.

import type { Application } from "pixi.js";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { commands } from "vitest/browser";

import { startLoopClock } from "../../../loop-clock";
import "../../../styles/index.css";
import { ConsoleStage } from "./console-stage";

const PLAN = { widths: [1, 1, 1.9, 1, 1], depths: [1, 1, 1.9, 1, 1], keyed: 2 };
const wait = (ms: number) => new Promise((done) => window.setTimeout(done, ms));

let host: HTMLElement;
let stage: ConsoleStage;
let stopClock: () => void = () => undefined;
const app = () => (stage as unknown as { app: Application | undefined }).app;
/** The frames drawn to the screen since this was called: a frame also renders each facet to its own texture first. */
const counting = () => {
  const render = vi.spyOn(app()!.renderer, "render");
  render.mockClear();
  return () => render.mock.calls.filter(([what]) => what === app()!.stage).length;
};

async function start(): Promise<void> {
  stage = ConsoleStage.start(host, PLAN, () => undefined);
  stage.resize(1000);
  await expect.poll(() => app() !== undefined && host.querySelector("canvas") !== null, { timeout: 60_000 }).toBe(true);
  stopClock = startLoopClock();
  await wait(300);
}

beforeEach(() => {
  host = document.createElement("div");
  host.style.cssText = "position:relative;width:1000px;height:300px";
  document.body.append(host);
});

afterEach(async () => {
  stopClock();
  document.documentElement.removeAttribute("data-idle");
  stage.destroy();
  host.remove();
  await commands.motion(false);
});

test("the lamps draw on the loop clock with the ticker stopped, and hold in an idle window", async () => {
  await commands.motion(false);
  await start();

  const drawn = counting();
  await wait(1100);
  // Five moves a second, and not the thirty a ticker would draw.
  expect(drawn()).toBeGreaterThanOrEqual(3);
  expect(drawn()).toBeLessThanOrEqual(7);
  expect(app()!.ticker.started).toBe(false);

  document.documentElement.setAttribute("data-idle", "");
  await wait(300);
  const held = counting();
  await wait(500);
  expect(held()).toBe(0);
});

test("reduced motion, turned on while it runs, stops the lamps and leaves the screens' once a second", async () => {
  await commands.motion(false);
  await start();

  await commands.motion(true);
  await expect.poll(() => (stage as unknown as { still: boolean }).still, { timeout: 5_000 }).toBe(true);
  const drawn = counting();
  await wait(2100);
  expect(drawn()).toBeGreaterThanOrEqual(1);
  expect(drawn()).toBeLessThanOrEqual(3);
});
