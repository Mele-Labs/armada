// The walk Bridge plays itself, on a page that knows nothing of Armada — `docs/practices/capture-window.md`,
// *Steps Bridge plays*. The player is driven here over a page of the test's own (a fake finder and a
// record of what was sent), so what is asserted is the order of the steps, what Back and Next do, when
// the walk holds, and that the session is told once.

import type { WalkStep, WalkTarget } from "@armada/protocol";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { CaptureRect } from "../../shared/capture-window";
import {
  beginsOn,
  DWELL_MS,
  GLIDE_MS,
  heldAt,
  PRESS_MS,
  keyEvents,
  PATIENCE_MS,
  pressEvents,
  saidOf,
  StepWalk,
  targetOf,
  typeEvents,
  type Sent,
  type StepHost,
} from "./stepwalk";

const BOX: CaptureRect = { x: 100, y: 200, width: 40, height: 20 };

const button = (name: string): WalkTarget => ({ role: "button", name });

/** Depot: look at the table, open the tab, type into the field, press Add, look at the status. */
const DEPOT: WalkStep[] = [
  { say: "The stock table", look: { role: "table", name: "Stock" } },
  { say: "Open the receiving tab", press: { role: "tab", name: "Receiving" } },
  { say: "Name the item", type: "Bolts\n", into: { role: "textbox", name: "Item" } },
  { say: "Add it", press: button("Add") },
  { say: "It is saved", look: { role: "status", name: "Saved" } },
];

/** A page: the names on it, and everything the player did to it. */
function page(on: Set<string>, steps: readonly WalkStep[] = DEPOT) {
  const log: string[] = [];
  const sent: Sent[] = [];
  const told: string[] = [];
  let changes = 0;
  let open = true;
  let words: { from: { x: number; y: number }; to: { x: number; y: number } } | null = { from: { x: 1, y: 2 }, to: { x: 9, y: 2 } };
  const nameOf = (target: WalkTarget) => target.name ?? target.role;
  let held: string | null = null;
  const host: StepHost = {
    finder: {
      arrive: vi.fn(async (target: WalkTarget) => {
        if (!on.has(nameOf(target))) return null;
        held = nameOf(target);
        log.push(`arrive ${held}`);
        return BOX;
      }),
      box: vi.fn(async () => (held !== null && on.has(held) ? BOX : null)),
      focus: vi.fn(async () => (held === "Item" ? "field" : "other")),
      words: vi.fn(async () => words),
      release: vi.fn(async () => undefined),
    } as unknown as StepHost["finder"],
    send: (one) => {
      sent.push(one);
    },
    command: (name) => void log.push(`command ${name}`),
    keyboard: (given) => void log.push(`keyboard ${given}`),
    reload: vi.fn(async () => {
      log.push("reload");
    }),
    changed: () => void (changes += 1),
    stuck: (why) => void told.push(why),
    open: () => open,
  };
  const walk = new StepWalk("Depot", steps, host);
  return {
    walk,
    host,
    log,
    sent,
    told,
    changes: () => changes,
    close: () => void (open = false),
    noWords: () => void (words = null),
  };
}

const ALL = new Set(["Stock", "Receiving", "Item", "Add", "Saved"]);
const kinds = (sent: Sent[]) => sent.map((one) => ("insert" in one ? "insert" : one.type));

/** Let the player's short sleeps and the finder's turns run. */
const settle = (ms = 800) => vi.advanceTimersByTimeAsync(ms);

beforeEach(() => vi.useFakeTimers());
afterEach(() => vi.useRealTimers());

describe("what the player says of a step", () => {
  it("the target a step aims at is its verb's own, whichever verb it is", () => {
    const aim = button("Add");
    expect(targetOf({ say: "s", press: aim })).toBe(aim);
    expect(targetOf({ say: "s", look: aim })).toBe(aim);
    expect(targetOf({ say: "s", hover: aim })).toBe(aim);
    expect(targetOf({ say: "s", later: aim })).toBe(aim);
    expect(targetOf({ say: "s", paste: aim })).toBe(aim);
    expect(targetOf({ say: "s", drag: aim, by: { x: 1, y: 1 } })).toBe(aim);
    expect(targetOf({ say: "s", type: "x", into: aim })).toBe(aim);
    expect(targetOf({ say: "s", key: "Tab", on: aim })).toBe(aim);
    expect(targetOf({ say: "s", select: "x", in: aim })).toBe(aim);
  });

  it("a target is said as a person would, and a nested one says where", () => {
    expect(saidOf({ role: "tab", name: "Receiving" })).toBe("a tab named “Receiving”");
    expect(saidOf({ role: "text", name: "Saved" })).toBe("the words “Saved”");
    expect(saidOf({ role: "button", name: "Close", within: { role: "dialog", name: "Settings" } })).toBe(
      "a button named “Close” in a dialog named “Settings”",
    );
  });

  it("the report names the step by its place, its caption and its target", () => {
    const said = heldAt(2, DEPOT, "target");
    expect(said).toMatch(/^step 3 of 5, “Name the item”: /);
    expect(said).toContain("textbox named “Item”");
    expect(said).toContain(`${PATIENCE_MS / 1000} seconds`);
    expect(heldAt(9, DEPOT, "target")).toBe("");
  });

  it("a held selection names the words and where they were meant to be", () => {
    const steps: WalkStep[] = [{ say: "Count", select: "box of bolts", in: { role: "text", name: "Twelve washers" } }];
    expect(heldAt(0, steps, "words")).toContain("“box of bolts” are not in the words “Twelve washers”");
  });

  it("a report is one bounded line", () => {
    const long: WalkStep[] = [{ say: "s\n".repeat(400), press: button("x".repeat(900)) }];
    const said = heldAt(0, long, "target");
    expect(said.length).toBeLessThanOrEqual(400);
    expect(said).not.toContain("\n");
  });
});

describe("what a step sends the page", () => {
  it("a press is a pointer arriving, going down and coming up at the target's middle", () => {
    const sent = pressEvents(BOX);
    expect(sent.map((one) => one.type)).toEqual(["mouseMove", "mouseDown", "mouseUp"]);
    expect(sent.every((one) => one.x === 120 && one.y === 210)).toBe(true);
  });

  it("typing is keys for what the keyboard has and text for what it has not, and a newline at the end is Enter", () => {
    const sent = typeEvents("Aé\n");
    expect(kinds(sent)).toEqual(["keyDown", "char", "keyUp", "insert", "keyDown", "char", "keyUp"]);
    expect((sent[0] as { modifiers?: string[] }).modifiers).toEqual(["shift"]);
    expect((sent[5] as { keyCode: string }).keyCode).toBe("Enter");
  });

  it("a chord is down, up, and no text while a command key is held", () => {
    expect(keyEvents("Meta+n").map((one) => one.type)).toEqual(["keyDown", "keyUp"]);
    expect(keyEvents("n").map((one) => one.type)).toEqual(["keyDown", "char", "keyUp"]);
    expect(keyEvents("ArrowDown")[0]).toMatchObject({ keyCode: "Down" });
  });
});

describe("the walk", () => {
  it("rings the first step and waits: it acts on nothing until Next", async () => {
    const depot = page(ALL);
    depot.walk.begin();
    await settle();
    expect(depot.walk.state()).toMatchObject({ name: "Depot", at: 1, of: 5, said: "The stock table", ready: true, done: false });
    expect(depot.walk.state().guide).toMatchObject({ box: BOX, folded: false });
    expect(depot.sent).toEqual([]);
  });

  it("plays the steps in order, one act on each Next, and ends done with the ring gone", async () => {
    const depot = page(ALL);
    depot.walk.begin();
    await settle();

    depot.walk.act("next"); // look: no input
    await settle();
    expect(depot.walk.state()).toMatchObject({ at: 2, said: "Open the receiving tab", ready: true });
    expect(depot.sent).toEqual([]);

    depot.walk.act("next"); // press the tab
    await settle();
    expect(kinds(depot.sent)).toEqual(["mouseMove", "mouseDown", "mouseUp"]);
    expect(depot.walk.state()).toMatchObject({ at: 3, said: "Name the item" });

    depot.sent.length = 0;
    depot.walk.act("next"); // type into the field: it is selected first, and the keyboard is given to the page
    await settle(1000);
    expect(depot.log).toContain("command selectAll");
    expect(depot.log.indexOf("keyboard true")).toBeLessThan(depot.log.indexOf("command selectAll"));
    expect(depot.log.at(-2)).toBe("keyboard false");
    expect(kinds(depot.sent).filter((one) => one === "char")).toHaveLength("Bolts".length + 1);
    expect(depot.walk.state()).toMatchObject({ at: 4, said: "Add it" });

    depot.walk.act("next"); // press Add
    await settle();
    depot.walk.act("next"); // look at the status
    await settle();
    const end = depot.walk.state();
    expect(end).toMatchObject({ done: true, ready: false, at: 5, of: 5 });
    expect(end.guide?.box).toBeNull();
    expect(depot.host.finder.release).toHaveBeenCalled();
    expect(depot.told).toEqual([]);
  });

  it("reports each press with where it landed, and presses once the bar's cursor has arrived", async () => {
    const depot = page(ALL);
    depot.walk.begin();
    await settle();
    expect(depot.walk.state().guide?.press).toBeUndefined();

    depot.walk.act("next"); // look
    await settle();
    expect(depot.walk.state().guide?.press).toBeUndefined();

    depot.walk.act("next"); // press the tab
    await settle(GLIDE_MS - 50);
    expect(depot.walk.state().guide?.press).toEqual({ x: 120, y: 210, n: 1 });
    expect(depot.sent).toEqual([]);
    await settle();
    expect(kinds(depot.sent)).toEqual(["mouseMove", "mouseDown", "mouseUp"]);

    depot.walk.act("next"); // type
    await settle(1000);
    depot.walk.act("next"); // press Add
    await settle();
    expect(depot.walk.state().guide?.press).toEqual({ x: 120, y: 210, n: 2 });

    // Once it has played out the bar is not asked to play it again.
    await settle(PRESS_MS);
    expect(depot.walk.state().guide?.press).toBeUndefined();
  });

  it("Back replays presses without showing the cursor", async () => {
    const depot = page(ALL);
    depot.walk.begin();
    await settle();
    depot.walk.act("next");
    await settle();
    depot.walk.act("next");
    await settle();
    depot.walk.act("next");
    await settle(1000);
    expect(depot.walk.state().at).toBe(4);
    await settle(PRESS_MS);

    depot.walk.act("back");
    await settle();
    expect(depot.walk.state().guide?.press).toBeUndefined();
  });

  it("a second Next while the first is acting is no second act", async () => {
    const depot = page(ALL);
    depot.walk.begin();
    await settle();
    depot.walk.act("next");
    await settle();
    depot.walk.act("next");
    depot.walk.act("next");
    await settle();
    expect(depot.sent.filter((one) => !("insert" in one) && one.type === "mouseDown")).toHaveLength(1);
    expect(depot.walk.state().at).toBe(3);
  });

  it("Back loads the page again and rings the step before, running everything earlier straight through", async () => {
    const depot = page(ALL);
    depot.walk.begin();
    await settle();
    depot.walk.act("next");
    await settle();
    depot.walk.act("next");
    await settle();
    expect(depot.walk.state().at).toBe(3);

    depot.sent.length = 0;
    depot.walk.act("back"); // from step 3 to step 2: step 1 is replayed (a look), step 2 is ringed and not pressed
    await settle();
    expect(depot.log.filter((one) => one === "reload")).toHaveLength(1);
    expect(depot.walk.state()).toMatchObject({ at: 2, said: "Open the receiving tab", ready: true });
    expect(depot.sent).toEqual([]);

    depot.walk.act("back"); // from step 2 to step 1
    await settle();
    expect(depot.walk.state()).toMatchObject({ at: 1, ready: true });
    depot.walk.act("back"); // Back on step 1 stays on step 1
    await settle();
    expect(depot.walk.state().at).toBe(1);
  });

  it("Replay starts from step 1 and replays the presses before the one it rings", async () => {
    const depot = page(ALL);
    depot.walk.begin();
    await settle();
    depot.walk.act("next");
    await settle();
    depot.walk.act("replay");
    await settle();
    expect(depot.walk.state()).toMatchObject({ at: 1, ready: true });
  });

  it("holds on a step whose target never comes, and tells the session which, once", async () => {
    const depot = page(new Set(["Stock", "Receiving"]));
    depot.walk.begin();
    await settle();
    depot.walk.act("next");
    await settle();
    depot.walk.act("next"); // presses the tab; step 3's field is not on the page
    await settle(PATIENCE_MS + 2000);

    const held = depot.walk.state();
    expect(held).toMatchObject({ at: 3, ready: false, done: false });
    expect(held.guide?.stopped).toContain("textbox named “Item”");
    expect(held.guide?.box).toBeNull();
    expect(depot.told).toHaveLength(1);
    expect(depot.told[0]).toMatch(/^step 3 of 5, “Name the item”/);

    // Held is held: Next does nothing, and time passing says nothing more.
    const sentBefore = depot.sent.length;
    depot.walk.act("next");
    await settle(PATIENCE_MS * 3);
    expect(depot.walk.state().at).toBe(3);
    expect(depot.sent).toHaveLength(sentBefore);
    expect(depot.told).toHaveLength(1);
  });

  it("goes on from a held step once the page has it, and a new run reports again", async () => {
    const present = new Set(["Stock", "Receiving"]);
    const depot = page(present);
    depot.walk.begin();
    await settle();
    depot.walk.act("next");
    await settle();
    depot.walk.act("next");
    await settle(PATIENCE_MS + 2000);
    expect(depot.told).toHaveLength(1);

    present.add("Item");
    depot.walk.act("replay");
    await settle(2000);
    expect(depot.walk.state()).toMatchObject({ at: 1, ready: true });
    expect(depot.told).toHaveLength(1);
    depot.walk.act("next");
    await settle();
    depot.walk.act("next");
    await settle(1000);
    expect(depot.walk.state()).toMatchObject({ at: 3, ready: true });

    present.delete("Item");
    depot.walk.act("replay");
    await settle(2000);
    depot.walk.act("next");
    await settle();
    depot.walk.act("next");
    await settle(PATIENCE_MS + 2000);
    expect(depot.told).toHaveLength(2);
    expect(depot.told[1]).toBe(depot.told[0]);
  });

  it("holds on a selection whose words are not in the target and says so", async () => {
    const steps: WalkStep[] = [{ say: "Count them", select: "bolts", in: { role: "text", name: "Twelve washers" } }];
    const depot = page(new Set(["Twelve washers"]), steps);
    depot.noWords();
    depot.walk.begin();
    await settle();
    depot.walk.act("next");
    await settle();
    expect(depot.walk.state().guide?.stopped).toContain("the words “bolts” in the words “Twelve washers”");
    expect(depot.told).toHaveLength(1);
    expect(depot.told[0]).toContain("are not in");
  });

  it("autoplay holds each step for the dwell and goes on by itself, and turning it off stops it", async () => {
    const depot = page(ALL);
    depot.walk.begin();
    await settle();
    depot.walk.act("autoplay_on");
    expect(depot.walk.state().autoplay).toBe(true);
    await settle(DWELL_MS + 600);
    expect(depot.walk.state().at).toBe(2);
    depot.walk.act("autoplay_off");
    await settle(DWELL_MS * 3);
    expect(depot.walk.state()).toMatchObject({ at: 2, autoplay: false });
  });

  it("folding the ring is the bar's to ask and tells the bar", async () => {
    const depot = page(ALL);
    depot.walk.begin();
    await settle();
    const before = depot.changes();
    depot.walk.guide(false);
    expect(depot.walk.state().guide?.folded).toBe(true);
    expect(depot.changes()).toBe(before + 1);
    depot.walk.guide(false);
    expect(depot.changes()).toBe(before + 1);
    depot.walk.guide(true);
    expect(depot.walk.state().guide?.folded).toBe(false);
  });

  it("a window that closes mid-walk lets everything go and says nothing", async () => {
    const depot = page(new Set());
    depot.walk.begin();
    await settle(1000);
    depot.close();
    depot.walk.stop();
    await settle(PATIENCE_MS * 2);
    expect(depot.told).toEqual([]);
    expect(depot.host.finder.release).toHaveBeenCalled();
  });
});

describe("when the steps begin", () => {
  it("on the first load of the shown page", () => {
    expect(beginsOn({ begun: false, signingIn: false, onShownOrigin: true })).toBe(true);
  });

  it("not on a sign-in address's load, though it is on the shown origin", () => {
    expect(beginsOn({ begun: false, signingIn: true, onShownOrigin: true })).toBe(false);
  });

  it("not on a provider's page the window passed through", () => {
    expect(beginsOn({ begun: false, signingIn: false, onShownOrigin: false })).toBe(false);
  });

  it("only once", () => {
    expect(beginsOn({ begun: true, signingIn: false, onShownOrigin: true })).toBe(false);
  });
});
