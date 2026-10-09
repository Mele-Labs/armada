import { describe, expect, test } from "vitest";

import { callsFromWaiting, permissionOf, sessionIdOf, waitingOf } from "./waiting";

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

  test("one card per item, the item that began first leading, and none for a Session at work or ended", () => {
    const two = session("two", {
      waitingFor: [
        { id: "ask:later", text: "b", since: "2026-10-09T09:30:00.000Z", source: "agent" },
        { id: "walk:first", text: "a", since: "2026-10-09T09:00:00.000Z", source: "walk" },
      ],
    });
    const working = session("busy", { turn: { state: "working" }, asked: { command: "x" } });
    const ended = session("gone", { dead: "ended", asked: { command: "y" } });
    expect(callsFromWaiting([two, working, ended]).map((one) => one.item.id)).toEqual(["walk:first", "ask:later"]);
  });

  test("a call's key names its Session, as the Session's own tile does", () => {
    expect(sessionIdOf("session:s9")).toBe("s9");
    expect(sessionIdOf("session:s9:ask:q-a:0")).toBe("s9");
    expect(sessionIdOf("job:s9")).toBeUndefined();
  });
});
