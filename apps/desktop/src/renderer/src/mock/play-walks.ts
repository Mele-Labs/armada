// Plays the checked-in walks as tests, one share of them per `walks-*.test.tsx`.
//
// **Split in four because one file is one worker.** All of them in one file
// took 175 s alone and was the long pole of every sharded run; shares of a
// quarter let a runner start four at once.

import { afterEach, beforeEach, expect, onTestFinished, test } from "vitest";
import { isNotice } from "@armada/shell";
import { page, userEvent } from "vitest/browser";

import { settle } from "../cockpit/CallAlerts";
import { mount, onScreen, unmountAfterEach } from "./testing";
import { walkThrough } from "./walk";
import type { Walk } from "./walk";
import { WALKS } from "./walks";

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

/**
 * The walks in share `part` of `of`, in the order `WALKS` holds them. **Longest
 * first into the lightest share**, by steps, so a walk added to `walks/` lands
 * in a share with no edit here and no share is left holding the long ones.
 */
export function walksIn(part: number, of: number): [string, Walk][] {
  const loads = Array.from({ length: of }, () => 0);
  const share = new Map<string, number>();
  const longestFirst = [...WALKS].sort(([a, one], [b, other]) => other.steps.length - one.steps.length || a.localeCompare(b));
  for (const [name, one] of longestFirst) {
    const lightest = loads.indexOf(Math.min(...loads));
    loads[lightest] = (loads[lightest] ?? 0) + one.steps.length;
    share.set(name, lightest + 1);
  }
  return [...WALKS].filter(([name]) => share.get(name) === part);
}

/**
 * The real pointer, moved to the window's corner. **A walk presses with
 * synthetic events, so it never moves it**, and a file run earlier in the same
 * worker can leave it resting where a plan option is drawn: that option then
 * previews as hovered and hides the Asker. `unhover` is not enough — it moves
 * to the middle of the page.
 */
export async function parkThePointer(): Promise<void> {
  await userEvent.hover(page.elementLocator(document.body), { position: { x: 0, y: 0 } });
}

/** Registers the tests of share `part` of `of`. Called once, at the top of a test file. */
export function playWalks(part: number, of: number): void {
  unmountAfterEach();

  const settling = settle.ms;
  beforeEach(async () => {
    // A walk is played faster than a person walks it, so the beat the first reading is given before a call can toast is spent.
    settle.ms = 0;
    await parkThePointer();
  });
  afterEach(() => {
    settle.ms = settling;
  });

  if (part === 1) {
    test("there is a walk to play", () => {
      expect(WALKS.size).toBeGreaterThan(0);
    });
  }

  for (const [name, script] of walksIn(part, of)) {
    test(`the walk ${name} plays to its last step`, async () => {
      const heard = thrown();
      // A walk that names its own window is played in it, and the next test gets its own back.
      if (script.viewport !== undefined) {
        await page.viewport(script.viewport.width, script.viewport.height);
        onTestFinished(() => page.viewport(1440, 900));
      }
      // The address names the walk, as the window that plays it does, so a page that reads it draws what the walk asks for.
      window.history.replaceState(null, "", `${window.location.pathname}?walk=${encodeURIComponent(name)}`);
      onTestFinished(() => window.history.replaceState(null, "", window.location.pathname));
      mount(script.scenario);
      await onScreen();
      await walkThrough(script.steps);
      expect(heard).toEqual([]);
      // A step waits for its target to settle, so a long walk outruns the default 15 s.
    }, Math.max(15_000, script.steps.length * 600));
  }
}
