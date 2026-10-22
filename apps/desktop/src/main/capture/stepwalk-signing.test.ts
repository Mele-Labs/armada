// What the player does on a deployed page, and while the person signs in — `docs/practices/capture-window.md`,
// *Steps Bridge plays*. The page is the test's own: a set of names that are on it, a record of what
// was sent, and a flag for whether it is off this machine.

import type { WalkStep, WalkTarget } from "@armada/protocol";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { CaptureRect } from "../../shared/capture-window";
import { DWELL_MS, PATIENCE_MS, StepWalk, type Sent, type StepHost } from "./stepwalk";

const BOX: CaptureRect = { x: 100, y: 200, width: 40, height: 20 };
const button = (name: string): WalkTarget => ({ role: "button", name });

/** Look, press, look, press, look: two steps that act, with a step that does not between them. */
const PORTAL: WalkStep[] = [
  { say: "The landing page", look: { role: "heading", name: "Welcome" } },
  { say: "Sign in", press: button("Sign in") },
  { say: "The dashboard", look: { role: "heading", name: "Dashboard" } },
  { say: "Open the report", press: button("Report") },
  { say: "The report", look: { role: "heading", name: "Report" } },
];
const ALL = ["Welcome", "Sign in", "Dashboard", "Report"];

function page(on: Set<string>, options: { guarded?: boolean } = {}) {
  const sent: Sent[] = [];
  const told: string[] = [];
  let held: string | null = null;
  const nameOf = (target: WalkTarget) => target.name ?? target.role;
  const host: StepHost = {
    finder: {
      arrive: vi.fn(async (target: WalkTarget) => {
        if (!on.has(nameOf(target))) return null;
        held = nameOf(target);
        return BOX;
      }),
      box: vi.fn(async () => (held !== null && on.has(held) ? BOX : null)),
      focus: vi.fn(async () => "other"),
      words: vi.fn(async () => null),
      release: vi.fn(async () => undefined),
    } as unknown as StepHost["finder"],
    send: (one) => void sent.push(one),
    command: () => {},
    keyboard: () => {},
    reload: vi.fn(async () => undefined),
    changed: () => {},
    stuck: (why) => void told.push(why),
    open: () => true,
    ...(options.guarded === undefined ? {} : { guarded: () => options.guarded === true }),
  };
  return { walk: new StepWalk("Portal", PORTAL, host), sent, told };
}

const settle = (ms = 800) => vi.advanceTimersByTimeAsync(ms);
const presses = (sent: Sent[]) => sent.filter((one) => !("insert" in one) && one.type === "mouseDown").length;

beforeEach(() => void vi.useFakeTimers());
afterEach(() => void vi.useRealTimers());

describe("while the page is being signed in on", () => {
  /** Press Sign in, so the walk looks for the dashboard, which a provider's page does not have. */
  async function pressedSignIn(on: Set<string>) {
    const portal = page(on);
    portal.walk.begin();
    await settle();
    portal.walk.act("next"); // the landing page
    await settle();
    portal.walk.act("next"); // Sign in
    await settle();
    return portal;
  }

  it("waits on the step past a step's patience, with no not-found and nothing told to the session", async () => {
    const portal = await pressedSignIn(new Set(["Welcome", "Sign in"]));
    portal.walk.signIn(true);
    await settle(PATIENCE_MS * 4);

    const state = portal.walk.state();
    expect(state).toMatchObject({ at: 3, ready: false, done: false });
    expect(state.guide).toMatchObject({ signingIn: true });
    expect(state.guide?.stopped).toBeUndefined();
    expect(portal.told).toEqual([]);
  });

  it("goes on at the same step, and rings it, once the page is back on the shown origin", async () => {
    const present = new Set(["Welcome", "Sign in"]);
    const portal = await pressedSignIn(present);
    portal.walk.signIn(true);
    await settle(PATIENCE_MS * 2);
    present.add("Dashboard");
    portal.walk.signIn(false);
    await settle(600);

    const state = portal.walk.state();
    expect(state).toMatchObject({ at: 3, ready: true });
    expect(state.guide?.signingIn).toBeUndefined();
    expect(portal.told).toEqual([]);
  });

  it("gives the step its whole patience again on the return", async () => {
    const portal = await pressedSignIn(new Set(["Welcome", "Sign in"]));
    portal.walk.signIn(true);
    await settle(PATIENCE_MS * 5);
    portal.walk.signIn(false);
    await settle(PATIENCE_MS - 1500);
    expect(portal.walk.state().guide?.stopped).toBeUndefined();
    expect(portal.told).toEqual([]);

    await settle(3000);
    expect(portal.walk.state().guide?.stopped).toContain("heading named “Dashboard”");
    expect(portal.told).toHaveLength(1);
  });

  it("shows the signing-in state, and not the not-found one, on a step that was already held, and looks again on the return", async () => {
    const present = new Set(["Welcome", "Sign in"]);
    const portal = await pressedSignIn(present);
    await settle(PATIENCE_MS + 2000);
    expect(portal.walk.state().guide?.stopped).toBeDefined();
    expect(portal.told).toHaveLength(1);

    portal.walk.signIn(true);
    expect(portal.walk.state().guide).toMatchObject({ signingIn: true });
    expect(portal.walk.state().guide?.stopped).toBeUndefined();

    present.add("Dashboard");
    portal.walk.signIn(false);
    await settle(600);
    expect(portal.walk.state()).toMatchObject({ at: 3, ready: true });
    expect(portal.told).toHaveLength(1);
  });

  it("is told to the bar only when it changes", async () => {
    const portal = await pressedSignIn(new Set(["Welcome", "Sign in"]));
    portal.walk.signIn(true);
    const before = vi.fn();
    (portal.walk as unknown as { host: StepHost }).host.changed = before;
    portal.walk.signIn(true);
    expect(before).not.toHaveBeenCalled();
    portal.walk.signIn(false);
    expect(before).toHaveBeenCalledTimes(1);
  });

  it("a walk that has not begun shows it too, and begins when the shown page loads", async () => {
    const present = new Set(["Welcome"]);
    const portal = page(present);
    portal.walk.signIn(true);
    expect(portal.walk.state().guide).toMatchObject({ signingIn: true });
    portal.walk.signIn(false);
    portal.walk.begin();
    await settle();
    expect(portal.walk.state()).toMatchObject({ at: 1, ready: true });
  });
});

describe("on a page off this machine", () => {
  it("Autoplay plays what does not act and stops before a step that does, waiting for Next", async () => {
    const portal = page(new Set(ALL), { guarded: true });
    portal.walk.begin();
    await settle();
    portal.walk.act("autoplay_on");
    await settle(DWELL_MS + 600); // the landing page is a look: on to Sign in
    expect(portal.walk.state().at).toBe(2);
    await settle(DWELL_MS * 3);

    const state = portal.walk.state();
    expect(state).toMatchObject({ at: 2, ready: true, autoplay: false });
    expect(state.guide).toMatchObject({ waiting: true });
    expect(presses(portal.sent)).toBe(0);
  });

  it("Next runs the step that waited, and Autoplay pressed again goes on to the next that acts", async () => {
    const portal = page(new Set(ALL), { guarded: true });
    portal.walk.begin();
    await settle();
    portal.walk.act("autoplay_on");
    await settle(DWELL_MS * 3);
    portal.walk.act("next");
    await settle();
    expect(presses(portal.sent)).toBe(1);
    expect(portal.walk.state()).toMatchObject({ at: 3, ready: true });
    expect(portal.walk.state().guide?.waiting).toBeUndefined();

    portal.walk.act("autoplay_on");
    await settle(DWELL_MS * 3);
    expect(portal.walk.state()).toMatchObject({ at: 4, autoplay: false });
    expect(portal.walk.state().guide).toMatchObject({ waiting: true });
    expect(presses(portal.sent)).toBe(1);
  });

  it("Back runs the steps before its target straight through until one that acts, which waits", async () => {
    const portal = page(new Set(ALL), { guarded: true });
    portal.walk.begin();
    await settle();
    for (const _ of [1, 2, 3]) {
      portal.walk.act("next");
      await settle();
    }
    expect(portal.walk.state().at).toBe(4);
    const before = presses(portal.sent);

    portal.walk.act("back"); // to step 3: step 2 presses Sign in, which a deployed page is not pressed again by itself
    await settle();
    expect(portal.walk.state()).toMatchObject({ at: 2, ready: true });
    expect(portal.walk.state().guide).toMatchObject({ waiting: true });
    expect(presses(portal.sent)).toBe(before);
  });

  it("Replay to step 1 rings it with nothing to wait for", async () => {
    const portal = page(new Set(ALL), { guarded: true });
    portal.walk.begin();
    await settle();
    portal.walk.act("next");
    await settle();
    portal.walk.act("replay");
    await settle();
    expect(portal.walk.state()).toMatchObject({ at: 1, ready: true });
    expect(portal.walk.state().guide?.waiting).toBeUndefined();
  });

  it("signing in wins over the waiting key", async () => {
    const portal = page(new Set(ALL), { guarded: true });
    portal.walk.begin();
    await settle();
    portal.walk.act("autoplay_on");
    await settle(DWELL_MS * 3);
    expect(portal.walk.state().guide).toMatchObject({ waiting: true });

    portal.walk.signIn(true);
    const guide = portal.walk.state().guide;
    expect(guide).toMatchObject({ signingIn: true });
    expect(guide?.waiting).toBeUndefined();
  });
});

describe("on a page on this machine", () => {
  it("Autoplay plays every step, the ones that act too", async () => {
    for (const guarded of [undefined, false]) {
      const portal = page(new Set(ALL), guarded === undefined ? {} : { guarded });
      portal.walk.begin();
      await settle();
      portal.walk.act("autoplay_on");
      await settle(DWELL_MS * 3 + 1500);
      expect(portal.walk.state().at).toBeGreaterThanOrEqual(4);
      expect(presses(portal.sent)).toBeGreaterThanOrEqual(1);
      expect(portal.walk.state().guide?.waiting).toBeUndefined();
    }
  });

  it("Back replays the presses before its target and rings the step with nothing to wait for", async () => {
    const portal = page(new Set(ALL), { guarded: false });
    portal.walk.begin();
    await settle();
    for (const _ of [1, 2, 3]) {
      portal.walk.act("next");
      await settle();
    }
    const before = presses(portal.sent);
    portal.walk.act("back");
    await settle();
    expect(portal.walk.state()).toMatchObject({ at: 3, ready: true });
    expect(portal.walk.state().guide?.waiting).toBeUndefined();
    expect(presses(portal.sent)).toBe(before + 1);
  });
});
