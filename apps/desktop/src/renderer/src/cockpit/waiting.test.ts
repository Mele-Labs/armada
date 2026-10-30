import { describe, expect, test } from "vitest";

import { callKindOf, callsFromWaiting, lastOwnerMessageAt, permissionOf, sessionIdOf, touchedAt, waitingOf } from "./waiting";

type Held = Parameters<typeof waitingOf>[0];

const session = (id: string, extra: Record<string, unknown>): Held =>
  ({ id, title: `Session ${id}`, turn: { state: "idle" }, lastTurnAt: "2026-10-09T10:00:00.000Z", rows: [], attachments: [], ...extra }) as unknown as Held;

const QUESTION = "The refresh ran on a stale build. Nothing you see changes either way. What should happen to it?";
const asking = (id: string): Held =>
  session(id, {
    asked: {
      command: "AskUserQuestion",
      call: `q-${id}`,
      questions: [
        {
          question: QUESTION,
          header: "Stale build",
          multi_select: false,
          options: [
            { label: "File an issue (Recommended)", description: "Write it up." },
            { label: "Fix it now", description: "" },
            { label: "Drop it", description: "Leave it." },
          ],
        },
      ],
    },
  });

describe("what a Session waits on", () => {
  test("a pending question is an ask_card item with the whole question and each option's own line", () => {
    const [item] = waitingOf(asking("a"));
    expect(item).toMatchObject({ id: "ask:q-a:0", source: "ask_card", text: QUESTION });
    expect(item?.options).toEqual([
      { label: "File an issue (Recommended)", description: "Write it up." },
      { label: "Fix it now" },
      { label: "Drop it", description: "Leave it." },
    ]);
  });

  test("a question with options never offers Allow or Deny", () => {
    for (const call of callsFromWaiting([asking("a")])) {
      const labels = (call.item.options ?? []).map((one) => one.label);
      expect(labels.some((one) => /allow|deny|refuse/i.test(one))).toBe(false);
    }
  });

  test("a permission offers what it was offered, or Allow once and Refuse where it says nothing", () => {
    expect(waitingOf(session("p", { asked: { command: "git push", call: "c1" } }))[0]).toMatchObject({ id: "perm:c1", source: "permission", text: "git push", options: [{ label: "Allow once" }, { label: "Refuse" }] });
    const offered = waitingOf(session("p", { asked: { command: "git push", call: "c2", offers: ["allow_once", "allow_and_remember", "refuse"] } }))[0];
    expect(offered?.options?.map((one) => one.label)).toEqual(["Allow once", "Allow and remember", "Refuse"]);
    expect(permissionOf("Allow and remember")).toBe("allow_and_remember");
    expect(permissionOf("Fix it now")).toBeUndefined();
  });

  test("the wire's own items win where the Session carries them", () => {
    const served = session("w", { asked: { command: "ignored" }, waitingFor: [{ id: "walk:u", text: "Look at the page", since: "2026-10-09T09:00:00.000Z", source: "walk", act: { kind: "walk", target: "u" } }] });
    expect(waitingOf(served)).toHaveLength(1);
    expect(waitingOf(served)[0]?.source).toBe("walk");
  });

  test("one card per item, the item that began first leading, and none for an ended Session", () => {
    const two = session("two", {
      waitingFor: [
        { id: "ask:later", text: "b", since: "2026-10-09T09:30:00.000Z", source: "agent" },
        { id: "walk:first", text: "a", since: "2026-10-09T09:00:00.000Z", source: "walk" },
      ],
    });
    const ended = session("gone", { dead: "ended", asked: { command: "y" } });
    expect(callsFromWaiting([two, ended]).map((one) => one.item.id)).toEqual(["walk:first", "ask:later"]);
  });

  // The owner, 12 Oct: "why the fuck are call cards from sessions not showing in cockpit". An agent
  // stopped on its question is mid-turn, so the Session reads `working` until he answers.
  test("a Session at work shows what its turn is stopped on, and nothing else", () => {
    const asking = session("asking", {
      turn: { state: "working" },
      waitingFor: [
        { id: "ask:q:0", text: "Two thoughts?", since: "2026-10-09T09:00:00.000Z", source: "ask_card" },
        { id: "walk:w", text: "a walk", since: "2026-10-09T09:10:00.000Z", source: "walk" },
      ],
    });
    const permission = session("perm", { turn: { state: "working" }, asked: { command: "rm -r x" } });
    expect(callsFromWaiting([asking, permission]).map((one) => one.item.id)).toEqual(["ask:q:0", "perm:perm"]);
  });

  test("a call's key names its Session, as the Session's own tile does", () => {
    expect(sessionIdOf("session:s9")).toBe("s9");
    expect(sessionIdOf("session:s9:ask:q-a:0")).toBe("s9");
    expect(sessionIdOf("job:s9")).toBeUndefined();
  });
});

// The owner, 13 Oct: a call card led him into a Session, he got it moving, and coming back to the
// cockpit the same card stood there stale. "I want these stale call cards to go away. Automatically."
describe("a call is raised only if he has not been in the Session since it began waiting", () => {
  const SINCE = "2026-10-09T09:00:00.000Z";
  const at = (iso: string) => Date.parse(iso);
  const item = (id: string, since: string, source = "agent") => ({ id, text: id, since, source });
  const waiting = (extra: Record<string, unknown> = {}) => session("w", { waitingFor: [item("a", SINCE)], ...extra });
  const youSaid = (iso: string) => ({ id: "m", at: iso, kind: "message", from: { kind: "you" }, text: "go on" });
  const ids = (sessions: Held[], viewedAt?: (id: string) => number) => callsFromWaiting(sessions, viewedAt).map((one) => one.item.id);

  test("he never was in it: a call", () => {
    expect(ids([waiting()])).toEqual(["a"]);
    expect(ids([waiting()], () => 0)).toEqual(["a"]);
  });

  test("an item older than his last view is no call", () => {
    expect(ids([waiting()], () => at("2026-10-09T09:30:00.000Z"))).toEqual([]);
  });

  test("an item older than his last message is no call", () => {
    expect(ids([waiting({ rows: [youSaid("2026-10-09T09:30:00.000Z")] })])).toEqual([]);
  });

  test("an item newer than both is a call", () => {
    const later = session("w", { rows: [youSaid("2026-10-09T09:30:00.000Z")], waitingFor: [item("b", "2026-10-09T10:00:00.000Z")] });
    expect(ids([later], () => at("2026-10-09T09:45:00.000Z"))).toEqual(["b"]);
  });

  test("whichever is later counts: his view after his message, and his message after his view", () => {
    const said = session("w", { rows: [youSaid("2026-10-09T09:30:00.000Z")] });
    expect(touchedAt(said, () => at("2026-10-09T09:40:00.000Z"))).toBe(at("2026-10-09T09:40:00.000Z"));
    expect(touchedAt(said, () => at("2026-10-09T09:10:00.000Z"))).toBe(at("2026-10-09T09:30:00.000Z"));
  });

  test("an item the agent posts again under the same id keeps its since and does not call again; a new one does", () => {
    const viewed = () => at("2026-10-09T09:30:00.000Z");
    const first = waiting();
    const again = waiting({ waitingFor: [item("a", SINCE), item("b", "2026-10-09T10:00:00.000Z")] });
    expect(ids([first], viewed)).toEqual([]);
    expect(ids([again], viewed)).toEqual(["b"]);
  });

  test("a message from the agent or another Session is not him", () => {
    const rows = [
      { id: "m1", at: "2026-10-09T09:30:00.000Z", kind: "message", from: { kind: "agent" }, text: "hi" },
      { id: "m2", at: "2026-10-09T09:31:00.000Z", kind: "message", from: { kind: "session", id: "x", title: "X" }, text: "hi" },
      { id: "t", at: "09:32:00", kind: "tool", text: "ls" },
    ];
    expect(lastOwnerMessageAt(waiting({ rows }))).toBe(0);
    expect(ids([waiting({ rows })])).toEqual(["a"]);
  });

  test("a reply that asks is a call, and a done beside it is one too", () => {
    const settled = session("w", { waitingFor: [item("replied:t", SINCE, "replied"), item("done:t", SINCE, "done")] });
    expect(ids([settled])).toEqual(["done:t", "replied:t"]);
    expect(waitingOf(settled).map((one) => one.id)).toContain("replied:t");
  });

  test("a reply that only tells has no item, so it raises no call", () => {
    expect(ids([session("t", { standing: "replied" })])).toEqual([]);
  });

  test("a row whose time is words, as a mock's, says nothing about when", () => {
    expect(lastOwnerMessageAt(waiting({ rows: [{ id: "m", at: "09:30:00", kind: "message", from: { kind: "you" }, text: "x" }] }))).toBe(0);
  });

  test("a blocking source follows the same rule: a question, a permission, a window ask, a full pool, the done ask on his tasks and a call a taught rule made are no call once he was there for them", () => {
    const working = (source: string) => session("w", { turn: { state: "working" }, waitingFor: [item(source, SINCE, source)] });
    for (const source of ["ask_card", "permission", "window_ask", "slot_choice", "delivery", "fleet_call", "task_done"]) {
      expect(ids([working(source)])).toEqual([source]);
      expect(ids([working(source)], () => at("2026-10-09T09:30:00.000Z"))).toEqual([]);
      expect(ids([working(source)], () => at("2026-10-09T08:30:00.000Z"))).toEqual([source]);
    }
  });

  test("a pending ask read off the Session follows it too, by the turn that ended", () => {
    expect(ids([asking("a")], () => at("2026-10-09T10:30:00.000Z"))).toEqual([]);
    expect(ids([asking("a")], () => at("2026-10-09T09:30:00.000Z"))).toEqual(["ask:q-a:0"]);
  });

  test("it is per Session: being in one leaves another's call", () => {
    const one = session("one", { waitingFor: [item("x", SINCE)] });
    const two = session("two", { waitingFor: [item("y", SINCE)] });
    expect(ids([one, two], (id) => (id === "one" ? at("2026-10-09T09:30:00.000Z") : 0))).toEqual(["y"]);
  });

  test("the item stays on the Session's own list: only the call goes", () => {
    const held = waiting();
    callsFromWaiting([held], () => at("2026-10-09T09:30:00.000Z"));
    expect(waitingOf(held).map((one) => one.id)).toEqual(["a"]);
  });
});

describe("a plan and a judge's concern", () => {
  const AT = "2026-10-16T09:00:00.000Z";
  const working = (extra: Record<string, unknown>) => session("w", { turn: { state: "working" }, ...extra });

  test("call the person while the turn goes on, since neither holds the turn", () => {
    const one = working({
      waitingFor: [
        { id: "plan:2", text: "Plan", since: AT, source: "plan" },
        { id: "c1", text: "No ceiling", since: AT, source: "judge" },
        { id: "q", text: "A line the agent stated", since: AT, source: "agent" },
      ],
    });
    expect(callsFromWaiting([one]).map((call) => call.item.id)).toEqual(["c1", "plan:2"]);
  });
});

describe("a draft put to the person", () => {
  const AT = "2026-10-26T09:00:00.000Z";

  test("calls him while the turn goes on, as a question does", () => {
    const one = session("w", { turn: { state: "working" }, waitingFor: [{ id: "draft:d1", text: "Description for the queue pull request", since: AT, source: "draft", options: [{ label: "Approve" }] }] });
    expect(callsFromWaiting([one]).map((call) => call.item.id)).toEqual(["draft:d1"]);
  });
});

describe("an act a rule would do, put to the person", () => {
  const AT = "2026-10-29T09:00:00.000Z";

  test("calls him while the turn goes on, as a draft does", () => {
    const item = { id: "rule_act:12", text: "Move ARM-41 to In progress", since: AT, source: "rule_act", act: { kind: "answer", target: "rule_act:12" }, options: [{ label: "Do it" }, { label: "Skip" }] };
    const one = session("w", { turn: { state: "working" }, waitingFor: [item] });
    expect(callsFromWaiting([one]).map((call) => call.item.id)).toEqual(["rule_act:12"]);
  });
});

describe("a sketch put to the person", () => {
  const AT = "2026-10-29T09:00:00.000Z";
  const sketch = { id: "sketch:Store clock", text: "Store clock", since: AT, source: "sketch", act: { kind: "message", target: "w" } };

  test("calls him while the turn goes on, and reads as a Job sketch", () => {
    const one = session("w", { turn: { state: "working" }, waitingFor: [sketch] });
    const calls = callsFromWaiting([one]);
    expect(calls.map((call) => call.item.id)).toEqual(["sketch:Store clock"]);
    expect(callKindOf("sketch")).toBe("Job sketch");
  });
});

describe("a Job whose agent never started", () => {
  const startFailed = { cause: "The agent exited at once", log: [] };

  test("waits on him for one item that opens the Job", () => {
    expect(waitingOf(session("f", { startFailed }))).toEqual([
      { id: "start_failed", text: "Start it again or report it", since: "2026-10-09T10:00:00.000Z", source: "first_message", act: { kind: "message", target: "f" } },
    ]);
  });

  test("comes ahead of the items the wire serves", () => {
    const owed = [{ id: "ask:1", text: "Which grid?", since: "2026-10-09T10:00:00.000Z", source: "ask_card" }];
    expect(waitingOf(session("f", { startFailed, waitingFor: owed })).map((one) => one.id)).toEqual(["start_failed", "ask:1"]);
  });

  test("waits on nothing once it has started", () => {
    expect(waitingOf(session("f", {}))).toEqual([]);
  });
});

describe("a Job he parked", () => {
  test("raises no call, and raises it again once it resumes", () => {
    const owed = [{ id: "ask:1", text: "Which grid?", since: "2026-10-09T10:00:00.000Z", source: "ask_card" }];
    expect(callsFromWaiting([session("a", { waitingFor: owed, paused: true })])).toEqual([]);
    expect(callsFromWaiting([session("a", { waitingFor: owed })]).map((one) => one.item.id)).toEqual(["ask:1"]);
  });
});
