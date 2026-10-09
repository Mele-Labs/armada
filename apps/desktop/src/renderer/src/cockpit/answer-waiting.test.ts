import { afterEach, describe, expect, test, vi } from "vitest";

import { answerWaiting } from "./answer-waiting";

const refusal = (code: string, message: string) => ({ ok: false as const, outcome: { ok: false as const, why: "refused" as const, error: { code, message, run_id: "", fields: {}, chain: [] } } });

/** `window.armada` with just the one route, answering as told. */
function fleetAnswering(answer: unknown) {
  const route = vi.fn(async () => answer);
  vi.stubGlobal("window", { armada: { answerWaiting: route } });
  return route;
}

afterEach(() => vi.unstubAllGlobals());

describe("answering what a Session waits on", () => {
  test("sends the request to Fleet whole: a choice is an index, and a mode stands alone", async () => {
    const route = fleetAnswering({ ok: true, value: {} });
    expect(await answerWaiting({ session_id: "s1", item_id: "ask:q1", choice: 2 })).toEqual({ kind: "answered" });
    expect(await answerWaiting({ session_id: "s1", item_id: "ask:q1", mode: "best" })).toEqual({ kind: "answered" });
    expect(route.mock.calls).toEqual([[{ session_id: "s1", item_id: "ask:q1", choice: 2 }], [{ session_id: "s1", item_id: "ask:q1", mode: "best" }]]);
  });

  test("an item nothing holds is gone, and says nothing", async () => {
    fleetAnswering(refusal("fleet.session_waiting_unheld", "nothing is waiting under that item."));
    expect(await answerWaiting({ session_id: "s1", item_id: "perm:c1", choice: 0 })).toEqual({ kind: "gone" });
  });

  test("any other refusal comes back in Fleet's words", async () => {
    fleetAnswering(refusal("fleet.session_waiting_empty", "an answer needs a choice, words or a mode"));
    expect(await answerWaiting({ session_id: "s1", item_id: "ask:q1" })).toEqual({ kind: "refused", said: "an answer needs a choice, words or a mode" });
  });

  test("a Fleet that is not there is a refusal too", async () => {
    fleetAnswering({ ok: false, outcome: { ok: false, why: "not_connected" } });
    const done = await answerWaiting({ session_id: "s1", item_id: "ask:q1", text: "yes" });
    expect(done.kind).toBe("refused");
  });
});
