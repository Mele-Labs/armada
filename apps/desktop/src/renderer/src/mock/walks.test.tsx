// Every checked-in walk, played as a test: a walk that stops matching the app
// fails here, rather than being found broken by the person it was sent to.
//
// **The walk's own engine plays it**, `walk.ts`, and not this suite's
// locators: what passes here is what the link does in his browser.
// A scratch walk, in `walks/scratch/`, is not here.

import { expect, onTestFinished, test } from "vitest";
import { isNotice } from "@armada/shell";
import { setPlanLayout } from "@armada/screens/src/plan-layout";

import { mount, onScreen, unmountAfterEach } from "./testing";
import { walkThrough } from "./walk";
import { WALKS } from "./walks";

unmountAfterEach();

/**
 * What the window threw while the walk played. **Heard before the app hears
 * it**: Bridge catches an uncaught throw and draws it as a banner, so a press
 * that broke a handler would otherwise walk on to the next step and pass.
 *
 * **A notice Bridge drops is dropped here too.** Chromium's ResizeObserver
 * loop notice is raised by any read-in landing on a Studio's board, and Bridge
 * reports it to nobody: `isNotice` in `packages/shell/src/uncaught.ts`.
 */
function thrown(): string[] {
  const heard: string[] = [];
  const hear = (event: ErrorEvent) => {
    if (!isNotice(event)) heard.push(event.message);
  };
  window.addEventListener("error", hear, true);
  onTestFinished(() => window.removeEventListener("error", hear, true));
  return heard;
}

test("there is a walk to play", () => {
  expect(WALKS.size).toBeGreaterThan(0);
});

for (const [name, script] of WALKS) {
  test(`the walk ${name} plays to its last step`, async () => {
    const heard = thrown();
    setPlanLayout(script.plan);
    onTestFinished(() => setPlanLayout(undefined));
    mount(script.scenario);
    await onScreen();
    await walkThrough(script.steps);
    expect(heard).toEqual([]);
  });
}
