// A changed row's mark fades over 45 seconds. Left alone it runs at the display's rate and redraws the whole window for all of them: one row read
// 34.9% GPU on the mock against 2.0%. Browser: the real stylesheet, so a renamed rule or a shorter token is caught here.

import { afterEach, expect, test } from "vitest";

import { startLoopClock } from "./loop-clock";
import "./styles/index.css";

const wait = (ms: number) => new Promise((done) => window.setTimeout(done, ms));
let stop: () => void = () => undefined;

afterEach(() => {
  stop();
  document.body.replaceChildren();
});

test("a changed row's fade runs on the clock, so it holds no frames of its own", async () => {
  const row = document.createElement("div");
  row.className = "armada-session-ledger__row";
  document.body.append(row);
  stop = startLoopClock();
  await wait(300);

  const [fade] = row.getAnimations();
  expect((fade as CSSAnimation).animationName).toBe("armada-session-arrive");
  expect(fade!.playState).toBe("paused");
  const at = Number(fade!.currentTime);
  await wait(500);
  expect(Number(fade!.currentTime)).toBeGreaterThan(at);
});
