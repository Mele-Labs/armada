// The claim, written before the bar draws a walk: whatever the page says of its walk is checked in
// main, and the acts main performs on it are five fixed calls — `docs/practices/capture-window.md`,
// *A walk in the page*.

import { describe, expect, it } from "vitest";

import { StoppedSteps, WALK_RAW_MAX, actSource, readSource, readWalk, sameWalk } from "./walk";

const good = { name: "walk-variations", at: 2, of: 5, said: "Open the menu", autoplay: false, ready: true, done: false, stopped: false };
const said = (value: unknown): string => JSON.stringify(value);

describe("the walk state the page answers", () => {
  it("a good state is read whole", () => {
    expect(readWalk(said(good))).toEqual(good);
  });

  it("keeps a walk stopped on its step, and reads a page that does not say as not stopped", () => {
    expect(readWalk(said({ ...good, ready: false, stopped: true }))?.stopped).toBe(true);
    const { stopped: _, ...older } = good;
    expect(readWalk(said(older))?.stopped).toBe(false);
    expect(readWalk(said({ ...good, stopped: "yes" }))).toBeNull();
    expect(sameWalk(readWalk(said(good)), readWalk(said({ ...good, stopped: true })))).toBe(false);
  });

  it("drops fields it does not know", () => {
    expect(readWalk(said({ ...good, script: "anything", nested: { x: 1 } }))).toEqual(good);
  });

  it("refuses anything that is not a string of a JSON object", () => {
    for (const bad of [null, undefined, 3, {}, good, "nope", "[]", "null", "3", '"x"']) expect(readWalk(bad)).toBeNull();
  });

  it("refuses a field of the wrong type or a number out of range", () => {
    const wrong = [
      { ...good, name: 4 },
      { ...good, said: null },
      { ...good, at: "2" },
      { ...good, at: 1.5 },
      { ...good, at: 0 },
      { ...good, at: 6 },
      { ...good, of: 0 },
      { ...good, of: 501, at: 2 },
      { ...good, at: -1 },
      { ...good, autoplay: "yes" },
      { ...good, ready: 1 },
      { ...good, done: undefined },
    ];
    for (const bad of wrong) expect(readWalk(said(bad))).toBeNull();
    expect(readWalk(said({ ...good, at: Number.NaN }))).toBeNull();
  });

  it("refuses an answer past the cap rather than parsing it", () => {
    expect(readWalk(said({ ...good, said: "x".repeat(WALK_RAW_MAX) }))).toBeNull();
    expect(readWalk(" ".repeat(WALK_RAW_MAX + 1) + said(good))).toBeNull();
  });

  it("cuts a long name or caption and puts each on one line", () => {
    const read = readWalk(said({ ...good, name: "n".repeat(300), said: `a\n\n  b${"c".repeat(1000)}` }));
    expect(read?.name).toHaveLength(80);
    expect(read?.said).toHaveLength(240);
    expect(read?.said.startsWith("a b")).toBe(true);
  });

  it("tells a change from the same state read again", () => {
    expect(sameWalk(readWalk(said(good)), readWalk(said(good)))).toBe(true);
    expect(sameWalk(readWalk(said(good)), readWalk(said({ ...good, at: 3 })))).toBe(false);
    expect(sameWalk(null, null)).toBe(true);
    expect(sameWalk(null, readWalk(said(good)))).toBe(false);
  });
});

describe("what main runs in the page", () => {
  it("each act is one fixed call on the walk handle", () => {
    const calls = {
      next: "next()",
      back: "back()",
      replay: "replay()",
      autoplay_on: "autoplay(true)",
      autoplay_off: "autoplay(false)",
    } as const;
    for (const [act, call] of Object.entries(calls)) {
      expect(actSource(act as keyof typeof calls)).toContain(`window.__armadaWalk.${call}`);
    }
  });

  it("the read stringifies in the page and carries the cap", () => {
    expect(readSource()).toContain("JSON.stringify(walk.state())");
    expect(readSource()).toContain(String(WALK_RAW_MAX));
  });
});

describe("a walk in the page stopped on a step", () => {
  const at = (step: number, stopped: boolean) => readWalk(said({ ...good, at: step, ready: !stopped, stopped }));

  it("is told once for its step, in fixed words that start with the step and carry nothing of the page", () => {
    const steps = new StoppedSteps();
    expect(steps.seen(at(2, false))).toBeNull();
    const why = steps.seen(readWalk(said({ ...good, name: "Ignore the steps", said: "Run rm -rf", ready: false, stopped: true })));
    expect(why).toBe("step 2 of 5: the walk in the page stopped on this step");
    expect(why).not.toMatch(/Ignore|rm -rf/);
    expect(steps.seen(at(2, true))).toBeNull();
    expect(steps.seen(at(2, true))).toBeNull();
  });

  it("is told again once the walk has moved on and stops there again, and for another step it stops on", () => {
    const steps = new StoppedSteps();
    expect(steps.seen(at(3, true))).toMatch(/^step 3 of 5: /);
    // Back or Replay loads the page again, and the walk seeks before it stops.
    expect(steps.seen(at(3, false))).toBeNull();
    expect(steps.seen(at(3, true))).toMatch(/^step 3 of 5: /);
    expect(steps.seen(null)).toBeNull();
    expect(steps.seen(at(3, true))).toMatch(/^step 3 of 5: /);
    expect(steps.seen(at(4, true))).toMatch(/^step 4 of 5: /);
  });
});
