// The message a ring's button sends, where it goes for a Session and for a Job, and when the button lets go.

import type { Outcome } from "@armada/protocol";
import type { SessionRow } from "@armada/screens/src/draft/sessions";
import { afterEach, expect, test, vi } from "vitest";

import { SENT_FOR_MS, UPDATE_ASK, askForUpdate, askedIn, forget, press, repliedSince, type UpdateApi } from "./ask-update";

const refused = { ok: false, why: "refused", error: { code: "x", message: "No Commander yet" } } as unknown as Outcome;

const api = (change: Partial<UpdateApi> = {}): UpdateApi & { sent: unknown[] } => {
  const sent: unknown[] = [];
  return {
    sent,
    sendSessionMessage: async (send) => {
      sent.push(["session", send]);
      return { ok: true };
    },
    askCommander: async (jobId, text) => {
      sent.push(["job", jobId, text]);
      return { ok: true } as Outcome;
    },
    ...change,
  };
};

afterEach(() => {
  forget("session:s1");
  forget("job:j1");
  vi.useRealTimers();
});

test("a Session is sent the words through the message call its composer sends with", async () => {
  const fleet = api();
  expect(await askForUpdate("session:s1", fleet)).toEqual({ ok: true });
  expect(fleet.sent).toEqual([["session", { session_id: "s1", text: "Progress update, please: what is done, what is running now, what is next, and anything blocked." }]]);
});

test("a Job's words go to its Commander", async () => {
  const fleet = api();
  expect(await askForUpdate("job:j1", fleet)).toEqual({ ok: true });
  expect(fleet.sent).toEqual([["job", "j1", UPDATE_ASK]]);
});

test("a refusal comes back in words, and a key that is neither is not implemented", async () => {
  expect(await askForUpdate("job:j1", api({ askCommander: async () => refused }))).toEqual({ ok: false, said: "No Commander yet" });
  expect(await askForUpdate("main", api())).toEqual({ ok: false, said: "Not implemented" });
});

test("a double press sends once, and a sent button sends nothing more", async () => {
  const fleet = api();
  const [first, second] = await Promise.all([press("session:s1", fleet), press("session:s1", fleet)]);
  expect([first, second]).toEqual([true, false]);
  expect(await press("session:s1", fleet)).toBe(false);
  expect(fleet.sent).toHaveLength(1);
});

test("a refused press can be pressed again", async () => {
  const fleet = api({ askCommander: async () => refused });
  expect(await press("job:j1", fleet)).toBe(true);
  expect(await press("job:j1", fleet)).toBe(true);
});

test("a press lets go after the time where no reply can be seen", async () => {
  vi.useFakeTimers();
  const fleet = api();
  await press("job:j1", fleet);
  expect(await press("job:j1", fleet)).toBe(false);
  vi.advanceTimersByTime(SENT_FOR_MS);
  expect(await press("job:j1", fleet)).toBe(true);
});

const row = (id: string, from: "you" | "agent", text: string): SessionRow => ({ id, at: "10:00", kind: "message", from: { kind: from }, text });

test("a Session has replied once an agent message follows the latest time the words are in its thread", () => {
  const asked = row("a", "you", UPDATE_ASK);
  expect(askedIn([asked, row("b", "agent", "Done: x")])).toBe(1);
  expect(repliedSince([], 0)).toBe(false);
  expect(repliedSince([asked], 0)).toBe(false);
  expect(repliedSince([asked, row("b", "agent", "Done: x")], 0)).toBe(true);
  // An earlier ask and its reply do not answer a later press whose row has not arrived.
  expect(repliedSince([asked, row("b", "agent", "Done: x")], 1)).toBe(false);
  expect(repliedSince([asked, row("b", "agent", "Done: x"), row("c", "you", UPDATE_ASK)], 1)).toBe(false);
  expect(repliedSince([asked, row("b", "agent", "Done: x"), row("c", "you", UPDATE_ASK), row("d", "agent", "Now: y")], 1)).toBe(true);
});
