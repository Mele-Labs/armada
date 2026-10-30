import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";

import { forgetViewed, markViewed } from "./cockpit/viewed";
import { holdsDelivery, stateOf } from "./session-state";

type Held = Parameters<typeof stateOf>[0];

const TURN_ENDED = "2026-10-09T10:00:00.000Z";
const at = Date.parse(TURN_ENDED);

const session = (extra: Record<string, unknown>): Held =>
  ({ id: "s1", title: "Pin the clock", turn: { state: "idle" }, lastTurnAt: TURN_ENDED, rows: [{ id: "m", at: "10:00:00", kind: "message", from: { kind: "agent" }, text: "Pinned." }], attachments: [], standing: "replied", ...extra }) as unknown as Held;

beforeEach(() => {
  const store = new Map<string, string>();
  vi.stubGlobal("window", {
    localStorage: { getItem: (key: string) => store.get(key) ?? null, setItem: (key: string, value: string) => void store.set(key, value), removeItem: (key: string) => void store.delete(key) },
  });
  forgetViewed();
});
afterEach(() => vi.unstubAllGlobals());

describe("a reply is unread until he has been in the Session since it ended", () => {
  test("never opened, it reads as an unread reply", () => {
    expect(stateOf(session({}))).toEqual({ state: "replied", said: "Unread reply" });
  });

  test("opened before the turn ended, it still does", () => {
    markViewed("s1", at - 1);
    expect(stateOf(session({})).state).toBe("replied");
  });

  test("opened after it, the Session stands as idle", () => {
    markViewed("s1", at + 1);
    expect(stateOf(session({}))).toEqual({ state: "idle", said: "Idle" });
  });

  test("a done is not read away by opening the Session", () => {
    markViewed("s1", at + 1);
    expect(stateOf(session({ standing: "done" })).state).toBe("done");
  });

  test("a reply that came with an item the agent stated is waiting on him, not a reply", () => {
    const stated = { id: "a", text: "Pick one", since: TURN_ENDED, source: "agent" };
    expect(stateOf(session({ waitingFor: [stated] })).state).toBe("waiting");
  });

  test("a question nobody answered, which Fleet keeps no item for, is waiting on him after he has read the reply", () => {
    markViewed("s1", at + 1);
    const lapsed = { id: "q", questions: [{ question: "Which night?", header: "Night", multi_select: false, options: [] }] };
    expect(stateOf(session({ lapsed }))).toEqual({ state: "waiting", said: "Waiting on you" });
  });
});

describe("working is what Fleet derived, not only a turn of its own", () => {
  test("a Session whose own turn ended, waiting on its subagents, says a subagent is working", () => {
    const running = { kind: "subagent", id: "sub-1", task: "Build the Fleet side of Kit's install", state: "running" };
    expect(stateOf(session({ standing: "working", attachments: [running] }))).toEqual({ state: "working", said: "Subagent working" });
  });

  test("a subagent that finished does not say so", () => {
    const done = { kind: "subagent", id: "sub-1", task: "Build the Fleet side of Kit's install", state: "done" };
    expect(stateOf(session({ standing: "working", attachments: [done] }))).toEqual({ state: "working", said: "Working" });
  });

  test("a terminal Session, which reports no turn of its own, is working while Fleet says so", () => {
    expect(stateOf(session({ turn: { state: "idle" }, standing: "working", lastTurnAt: undefined }))).toEqual({ state: "working", said: "Working" });
  });
});

describe("the delivery question holds the Session on him", () => {
  test("a Session at work with the question on it is waiting on him, whatever its turn says", () => {
    const options = [{ label: "Auto land" }, { label: "Open PR only" }, { label: "Keep local" }];
    const question = { id: "delivery", text: "How should this work be delivered?", since: TURN_ENDED, source: "delivery", options };
    expect(stateOf(session({ turn: { state: "working" }, standing: "working", waitingFor: [question] }))).toEqual({ state: "waiting", said: "Waiting on you" });
  });
});

describe("a plan put to him holds the Session on him", () => {
  test("a Session at work with the plan on it is waiting on him, whatever its turn says", () => {
    const plan = { id: "plan:1", text: "Plan to approve", since: TURN_ENDED, source: "plan", options: [] };
    expect(stateOf(session({ turn: { state: "working" }, standing: "working", waitingFor: [plan] }))).toEqual({ state: "waiting", said: "Waiting on you" });
  });
});

describe("the delivery question is what a new Job's Needs you card opens on", () => {
  const options = [{ label: "Auto land" }, { label: "Open PR only" }, { label: "Keep local" }];
  const delivery = { id: "delivery", text: "How should this work be delivered?", since: TURN_ENDED, source: "delivery", options };
  const slot = { id: "slot-choice", text: "Every slot is in use", since: TURN_ENDED, source: "slot_choice", options: [{ label: "Wait for a slot" }] };

  test("a Session waiting on the delivery question holds the card open", () => {
    expect(holdsDelivery(session({ waitingFor: [delivery] }))).toBe(true);
  });

  test("a Session waiting on anything else does not", () => {
    expect(holdsDelivery(session({ waitingFor: [slot] }))).toBe(false);
    expect(holdsDelivery(session({}))).toBe(false);
  });
});

describe("a full pool holds the Session on him", () => {
  test("a Session at work with the pool's call on it is waiting on him, whatever its turn says", () => {
    const choice = { id: "slot-choice", text: "Every slot is in use", since: TURN_ENDED, source: "slot_choice", options: [{ label: "Wait for a slot" }, { label: "Add a slot" }] };
    expect(stateOf(session({ turn: { state: "working" }, standing: "working", waitingFor: [choice] }))).toEqual({ state: "waiting", said: "Waiting on you" });
  });
});

describe("a Job held on its start card is a draft", () => {
  const first = { id: "unsent:s1", text: "First message", since: TURN_ENDED, source: "first_message", options: [] };
  const start = { asksDelivery: true, message: "Read the loader." };

  test("holding its first message makes it a draft, ahead of what it waits on him for", () => {
    expect(stateOf(session({ start, standing: "idle", lastTurnAt: undefined, rows: [], waitingFor: [first] }))).toEqual({ state: "draft", said: "Not started" });
  });

  test("while its request is read, it is reading", () => {
    expect(stateOf(session({ start, work: { proposal: { reading: true } } })).state).toBe("working");
  });

  test("once begun it is not", () => {
    expect(stateOf(session({ turn: { state: "working" }, standing: "working" })).state).toBe("working");
  });
});

describe("a Job whose agent never started", () => {
  const startFailed = { cause: "The agent exited at once", log: ["error: no such option"] };

  test("reads as unstarted and says why", () => {
    expect(stateOf(session({ startFailed }))).toEqual({ state: "unstarted", said: "Failed to start: The agent exited at once" });
  });

  test("paused, ended and not reachable still win over it", () => {
    expect(stateOf(session({ startFailed, paused: true })).state).toBe("paused");
    expect(stateOf(session({ startFailed, dead: "ended" })).state).toBe("ended");
    expect(stateOf(session({ startFailed, dead: "quiet" })).state).toBe("quiet");
  });

  test("once it has started it reads as it did", () => {
    expect(stateOf(session({})).state).not.toBe("unstarted");
  });
});

describe("the done ask on his project tasks holds the Session on him", () => {
  test("a Session at work with a task_done item on it is waiting on him, whatever its turn says", () => {
    const ask = { id: "task_done:p1", text: "Mark Sign the build done?", since: TURN_ENDED, source: "task_done", options: [{ label: "Mark done" }, { label: "Keep open" }] };
    expect(stateOf(session({ turn: { state: "working" }, standing: "working", waitingFor: [ask] }))).toEqual({ state: "waiting", said: "Waiting on you" });
  });
});

describe("a sketch put to him holds the Session on him", () => {
  test("a Session at work with a sketch item on it is waiting on him, whatever its turn says", () => {
    const sketch = { id: "sketch:Store clock", text: "Store clock", since: TURN_ENDED, source: "sketch", act: { kind: "message", target: "s1" } };
    expect(stateOf(session({ turn: { state: "working" }, standing: "working", waitingFor: [sketch] }))).toEqual({ state: "waiting", said: "Waiting on you" });
  });

  test("so does a document put to him, whatever its turn says", () => {
    const page = { id: "document:/work/.armada/documents/script.html", text: "Video script", since: TURN_ENDED, source: "document", act: { kind: "message", target: "s1" } };
    expect(stateOf(session({ turn: { state: "working" }, standing: "working", waitingFor: [page] }))).toEqual({ state: "waiting", said: "Waiting on you" });
  });
});

describe("a Job he parked", () => {
  const paused = { paused: true, waitingFor: [{ id: "ask:1", text: "Which grid?", since: TURN_ENDED, source: "ask_card" }] };

  test("reads as paused ahead of waiting, done and failing", () => {
    expect(stateOf(session({ ...paused }))).toEqual({ state: "paused", said: "Paused" });
    expect(stateOf(session({ paused: true, standing: "done" })).state).toBe("paused");
    expect(stateOf(session({ paused: true, attachments: [{ kind: "pull_request", number: 7, title: "t", branch: "b", address: "", checks: { state: "failed", failing: "ci" }, state: "open", auto: false }] })).state).toBe("paused");
  });

  test("a Job that ended or is not reachable reads as that", () => {
    expect(stateOf(session({ paused: true, dead: "ended" })).state).toBe("ended");
    expect(stateOf(session({ paused: true, dead: "quiet" })).state).toBe("quiet");
  });

  test("resumed, it waits on him again", () => {
    expect(stateOf(session({ waitingFor: paused.waitingFor })).state).toBe("waiting");
  });
});
