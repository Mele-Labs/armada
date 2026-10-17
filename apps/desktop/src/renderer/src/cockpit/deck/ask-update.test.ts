// The message a ring's button sends, where it goes for a Session, and when the button lets go.

import type { SessionRow, SessionsDraft } from "@armada/screens/src/draft/sessions";
import { afterEach, expect, test, vi } from "vitest";

import { ASK_FOR_UPDATE, askForUpdate } from "./acts";
import { SENT_FOR_MS, askRing, askedIn, forget, press, repliedSince, type UpdateApi } from "./ask-update";

const api = (took: boolean | void = true): UpdateApi & { sent: unknown[] } => {
  const sent: unknown[] = [];
  const draft = {
    send: async (id: string, message: unknown) => {
      sent.push(["session", id, message]);
      return took;
    },
  } as unknown as SessionsDraft;
  return { sent, draft };
};

afterEach(() => {
  forget("session:s1");
  vi.useRealTimers();
});

test("a Session is sent the words through the composer's send, with nothing attached", async () => {
  const fleet = api();
  expect(await askRing("session:s1", fleet)).toEqual({ ok: true });
  expect(fleet.sent).toEqual([["session", "s1", { text: "Bring your task list up to date with TaskUpdate, using the plan's task text word for word if you recorded a plan, then say what is done, what is running now, what is next, and anything blocked.", files: [], sketches: [], tags: [] }]]);
  expect(ASK_FOR_UPDATE).toBe("Bring your task list up to date with TaskUpdate, using the plan's task text word for word if you recorded a plan, then say what is done, what is running now, what is next, and anything blocked.");
});

test("askForUpdate answers whether the send was taken, and false for no draft", async () => {
  expect(await askForUpdate(api().draft, "s1")).toBe(true);
  expect(await askForUpdate(api(void 0).draft, "s1")).toBe(true);
  expect(await askForUpdate(api(false).draft, "s1")).toBe(false);
  expect(await askForUpdate(undefined, "s1")).toBe(false);
});

test("a refusal comes back in words, and a key that is not a Session's is not implemented", async () => {
  expect(await askRing("session:s1", api(false))).toEqual({ ok: false, said: "Update not sent" });
  expect(await askRing("main", api())).toEqual({ ok: false, said: "Not implemented" });
});

test("a double press sends once, and a sent button sends nothing more", async () => {
  const fleet = api();
  const [first, second] = await Promise.all([press("session:s1", fleet), press("session:s1", fleet)]);
  expect([first, second]).toEqual([true, false]);
  expect(await press("session:s1", fleet)).toBe(false);
  expect(fleet.sent).toHaveLength(1);
});

test("a refused press can be pressed again", async () => {
  const fleet = api(false);
  expect(await press("session:s1", fleet)).toBe(true);
  expect(await press("session:s1", fleet)).toBe(true);
});

test("a press lets go after the time where no reply can be seen", async () => {
  vi.useFakeTimers();
  const fleet = api();
  await press("session:s1", fleet);
  expect(await press("session:s1", fleet)).toBe(false);
  vi.advanceTimersByTime(SENT_FOR_MS);
  expect(await press("session:s1", fleet)).toBe(true);
});

const row = (id: string, from: "you" | "agent", text: string): SessionRow => ({ id, at: "10:00", kind: "message", from: { kind: from }, text });

test("a Session has replied once an agent message follows the latest time the words are in its thread", () => {
  const asked = row("a", "you", ASK_FOR_UPDATE);
  expect(askedIn([asked, row("b", "agent", "Done: x")])).toBe(1);
  expect(repliedSince([], 0)).toBe(false);
  expect(repliedSince([asked], 0)).toBe(false);
  expect(repliedSince([asked, row("b", "agent", "Done: x")], 0)).toBe(true);
  // An earlier ask and its reply do not answer a later press whose row has not arrived.
  expect(repliedSince([asked, row("b", "agent", "Done: x")], 1)).toBe(false);
  expect(repliedSince([asked, row("b", "agent", "Done: x"), row("c", "you", ASK_FOR_UPDATE)], 1)).toBe(false);
  expect(repliedSince([asked, row("b", "agent", "Done: x"), row("c", "you", ASK_FOR_UPDATE), row("d", "agent", "Now: y")], 1)).toBe(true);
});
