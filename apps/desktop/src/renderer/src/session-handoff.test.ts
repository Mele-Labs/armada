// The handoff a fresh Session opens on: the old Session's own reply to being asked for one, and the ledger's
// facts where it cannot answer. The draft is a bare stand-in for what `sessions-wired.tsx` serves.

import { afterEach, beforeEach, expect, test, vi } from "vitest";
import type { Session, SessionRow, SessionsDraft } from "@armada/screens/src/draft/sessions";

import { askForHandoff, HANDOFF_ASK, handoffFromLedger, handoffOf } from "./session-handoff";

const AT = "2026-10-13T10:00:00.000Z";

function session(change: Partial<Session> = {}): Session {
  return { id: "s1", address: "s-s1", title: "Cart totals rounding", attachments: [], rows: [], turn: { state: "idle" }, pendingTags: [], ...change } as Session;
}

/** A draft holding one Session, whose `send` is `onSend` and which tells every subscriber when it changes. */
function drafted(first: Session, onSend: (held: { set: (next: Session) => void; get: () => Session }) => boolean | undefined | Promise<boolean>) {
  let held = first;
  const heard = new Set<() => void>();
  const set = (next: Session) => {
    held = next;
    heard.forEach((on) => on());
  };
  const sent: string[] = [];
  const draft = {
    get: () => [held],
    subscribe: (on: () => void) => {
      heard.add(on);
      return () => void heard.delete(on);
    },
    send: async (_id: string, message: { text: string }) => {
      sent.push(message.text);
      return onSend({ set, get: () => held });
    },
  } as unknown as SessionsDraft;
  return { draft, sent, set, get: () => held };
}

const you = (text: string): SessionRow => ({ id: `y${text.length}`, at: AT, kind: "message", from: { kind: "you" }, text });
const agent = (id: string, text: string): SessionRow => ({ id, at: AT, kind: "message", from: { kind: "agent" }, text });

beforeEach(() => vi.useFakeTimers());
afterEach(() => vi.useRealTimers());

test("The ask goes to the old Session and its reply, once the turn is over, is the handoff", async () => {
  const one = drafted(session(), ({ set, get }) => {
    // The ask lands in the thread and the agent starts; its reply arrives, and then the turn ends.
    set({ ...get(), rows: [...get().rows, you(HANDOFF_ASK)], turn: { state: "working" } });
    set({ ...get(), rows: [...get().rows, agent("a1", "Goal: totals match.")], turn: { state: "working" } });
    set({ ...get(), turn: { state: "idle" } });
    return true;
  });

  expect(await askForHandoff(one.draft, "s1")).toBe("Goal: totals match.");
  expect(one.sent).toEqual([HANDOFF_ASK]);
});

test("A reply written while the turn still runs is not taken until it ends, and the last one stands", async () => {
  const one = drafted(session(), ({ set, get }) => {
    set({ ...get(), rows: [...get().rows, you(HANDOFF_ASK), agent("a1", "Let me look.")], turn: { state: "working" } });
    return true;
  });
  const asked = askForHandoff(one.draft, "s1");
  await vi.advanceTimersByTimeAsync(1_000);
  one.set({ ...one.get(), rows: [...one.get().rows, agent("a2", "Goal: totals match.")] });
  one.set({ ...one.get(), turn: { state: "idle" } });

  expect(await asked).toBe("Goal: totals match.");
});

test("An earlier handoff in the thread is not mistaken for the answer to this ask", async () => {
  const earlier = [you(HANDOFF_ASK), agent("a0", "Goal: the old one.")];
  // The new ask has not reached the thread yet, and the turn the old one started is long over.
  const one = drafted(session({ rows: earlier }), () => true);
  const asked = askForHandoff(one.draft, "s1", 5_000);
  await vi.advanceTimersByTimeAsync(5_001);

  expect(await asked).toBeUndefined();
});

test("A refused send, an ended Session and a silence past the bound each give no handoff", async () => {
  const refused = drafted(session(), () => false);
  expect(await askForHandoff(refused.draft, "s1")).toBe(undefined);

  const ended = drafted(session(), ({ set, get }) => {
    set({ ...get(), dead: "ended" });
    return true;
  });
  expect(await askForHandoff(ended.draft, "s1")).toBe(undefined);

  const silent = drafted(session(), () => true);
  const waited = askForHandoff(silent.draft, "s1", 90_000);
  await vi.advanceTimersByTimeAsync(90_001);
  expect(await waited).toBe(undefined);
});

test("The ledger stands in: the title, the slot, the branches and the open pull requests", () => {
  const text = handoffFromLedger(
    session({
      attachments: [
        { kind: "slot", slot: 3 },
        { kind: "branch", name: "fix/cart-totals", slot: 3 },
        { kind: "pull_request", number: 12345, title: "Round once", branch: "fix/cart-totals", address: "u", checks: { passed: 0, failed: 0, running: 0 }, state: "open", auto: false },
        { kind: "pull_request", number: 398, title: "Older", branch: "fix/older", address: "u", checks: { passed: 0, failed: 0, running: 0 }, state: "merged", auto: false },
      ],
    } as Partial<Session>),
  );

  expect(text).toBe("Handoff from Cart totals rounding\nSlot 3\nBranch fix/cart-totals\nPull request #12345 fix/cart-totals");
});

test("With no answer the first message is the ledger's, and with one it is the Session's own words under a heading", async () => {
  const silent = drafted(session({ attachments: [{ kind: "slot", slot: 3 }] }), () => false);
  expect(await handoffOf(silent.draft, session({ attachments: [{ kind: "slot", slot: 3 }] }))).toBe("Handoff from Cart totals rounding\nSlot 3");

  const writes = drafted(session(), ({ set, get }) => {
    set({ ...get(), rows: [...get().rows, you(HANDOFF_ASK), agent("a1", "Goal: totals match.")] });
    return true;
  });
  expect(await handoffOf(writes.draft, session())).toBe("Handoff from Cart totals rounding\n\nGoal: totals match.");
});
