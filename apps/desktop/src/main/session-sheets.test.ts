// A calling Work row pulses a bar by its opacity alone, and holds still where motion is reduced. The pinned calls' scroll-driven shrink is gone
// from the thread's sheet whole. Read off the sheets: a headless run cannot tell a pulse from a still frame.

import { readFileSync } from "node:fs";
import { expect, test } from "vitest";

const ROOT = new URL("../../../../", import.meta.url).pathname;
const read = (path: string) => readFileSync(ROOT + path, "utf8").replace(/\/\*[\s\S]*?\*\//g, "");
const sheet = read("apps/desktop/src/renderer/src/session-view/session-view.css");
const thread = read("packages/components/src/compositions/SessionThread/SessionThread.css");

test("the pulse moves only the bar's opacity, and stands still under reduced motion", () => {
  const keyframes = /@keyframes deck-call-pulse\s*\{([^]*?)\}\s*\}/.exec(sheet)?.[1] ?? "";
  expect(keyframes).toContain("opacity");
  expect(keyframes).not.toMatch(/transform|translate|scale|rotate|width|height|top|left/);
  expect(sheet).toMatch(/\.deck-row\[data-calling\]::before\s*\{[^}]*animation:\s*deck-call-pulse[^}]*infinite/);
  expect(sheet).toMatch(/@media \(prefers-reduced-motion: reduce\)\s*\{\s*\.deck-row\[data-calling\]::before\s*\{\s*animation:\s*none;?\s*\}/);
});

test("nothing of the pinned region's scroll-driven shrink is left in the thread's sheet", () => {
  for (const gone of ["--thread-scroll", "armada-session-calls-shrink", "--calls-body", "scroll-timeline", "animation-timeline", "armada-session-thread__calls"]) expect(thread, gone).not.toContain(gone);
});

test("the sheets carry no rule gated on the new look, which the restyle owns", () => {
  expect(sheet).not.toContain('data-look="new"');
});
