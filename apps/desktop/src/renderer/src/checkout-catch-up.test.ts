// "Your checkout" run against a real Fleet: what Fleet answers plays as a run does, and the card's command string is never sent.

import { describe, expect, it, vi } from "vitest";
import type { CheckoutCaughtUp } from "@armada/protocol";

import type { SessionActed } from "../../shared/api/sessions";
import { catchingUp } from "./checkout-catch-up";

type Event = { kind: "out"; text: string } | { kind: "end"; code: number | null };

/** Plays one catch-up to its end and returns what the block would have drawn. */
async function played(answer: SessionActed<CheckoutCaughtUp>, manifestId: string | undefined = "01MANIFEST") {
  const events: Event[] = [];
  const asked = vi.fn(async (_manifestId: string) => answer);
  catchingUp(asked, manifestId, (event) => events.push(event));
  await vi.waitFor(() => expect(events.some((event) => event.kind === "end")).toBe(true));
  return { events, asked };
}

describe("a catch-up pressed on a held checkout", () => {
  it("asks Fleet by the repository's Manifest and by nothing else", async () => {
    const { asked } = await played({ ok: true, value: { code: 0, output: "Fast-forward" } });
    expect(asked).toHaveBeenCalledTimes(1);
    expect(asked.mock.calls[0]).toEqual(["01MANIFEST"]);
  });

  it("plays git's words as output, then ends with git's code", async () => {
    const { events } = await played({ ok: true, value: { code: 0, output: "Updating 1..2\nFast-forward" } });
    expect(events).toEqual([
      { kind: "out", text: "Updating 1..2\r\nFast-forward\r\n" },
      { kind: "end", code: 0 },
    ]);
  });

  it("ends with a non-zero code when the pull left the stash conflicted", async () => {
    const { events } = await played({ ok: true, value: { code: 1, output: "Your local changes are stashed" } });
    expect(events.at(-1)).toEqual({ kind: "end", code: 1 });
    expect(events[0]).toEqual({ kind: "out", text: "Your local changes are stashed\r\n" });
  });

  it("ends without output when git printed nothing", async () => {
    const { events } = await played({ ok: true, value: { code: 0, output: "" } });
    expect(events).toEqual([{ kind: "end", code: 0 }]);
  });

  it("plays Fleet's refusal as its words and ends with 1", async () => {
    const { events } = await played({
      ok: false,
      outcome: {
        ok: false,
        why: "refused",
        error: { code: "fleet.checkout_not_catchable", message: "the checkout is on `x` and not `main`", run_id: "r", fields: {}, chain: [] },
      },
    });
    expect(events).toEqual([
      { kind: "out", text: "the checkout is on `x` and not `main`\r\n" },
      { kind: "end", code: 1 },
    ]);
  });

  it("plays a Fleet that is not reached as a failed run", async () => {
    const { events } = await played({ ok: false, outcome: { ok: false, why: "not_connected" } });
    expect(events.at(-1)).toEqual({ kind: "end", code: 1 });
  });

  it("asks nothing of a repository with no Manifest, and says so", () => {
    const events: Event[] = [];
    const asked = vi.fn();
    catchingUp(asked, undefined, (event) => events.push(event));
    expect(asked).not.toHaveBeenCalled();
    expect(events.at(-1)).toEqual({ kind: "end", code: 1 });
  });
});
