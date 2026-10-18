// What a reply is on the deck: a reply that asks him something is a `replied` item and a call under Needs you; a reply that only
// tells has no item, raises no call, and is calm with a quiet mark.

import type { Session } from "@armada/screens/src/draft/sessions";
import { expect, test } from "vitest";

import { callsFromWaiting } from "../waiting";
import { groupOf } from "./order";
import { shipOf } from "./ship";

const SINCE = "2026-10-18T09:00:00.000Z";

const reply = (id: string, extra: Partial<Session> = {}): Session => ({
  id,
  title: `Session ${id}`,
  turn: { state: "idle" },
  lastTurnAt: SINCE,
  standing: "replied",
  rows: [{ id: `${id}-a`, at: "09:00:00", kind: "message", from: { kind: "agent" }, text: "The retry loop is capped." }],
  attachments: [],
  ...extra,
});

const asks = reply("asks", { waitingFor: [{ id: `replied:${SINCE}`, text: "Should the cap be five or ten?", since: SINCE, source: "replied" }] });
const tells = reply("tells");

test("a reply that asks raises a call and sits under Needs you, and a reply that only tells raises none", () => {
  expect(callsFromWaiting([asks, tells]).map((one) => [one.session.id, one.item.text])).toEqual([["asks", "Should the cap be five or ten?"]]);

  const ship = shipOf({ sessions: [asks, tells], lines: [], slots: [], repositories: [], calls: [], now: Date.parse(SINCE) + 60_000 });
  const of = (id: string) => ship.subjects.find((one) => one.id === id)!;
  expect(of("asks").needs).toBe(true);
  expect(of("asks").call).toBeDefined();
  expect(groupOf(of("asks"))).toBe("needs");
  expect(of("tells").needs).toBe(false);
  expect(of("tells").call).toBeUndefined();
  expect(of("tells").reply).toBe(true);
  expect(groupOf(of("tells"))).not.toBe("needs");
  expect(ship.calls.map((one) => one.key)).toEqual([expect.stringContaining("session:asks")]);
});
