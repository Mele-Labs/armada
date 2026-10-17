// What `proposeFromRequest` answered, and whether anything is said about it.
//
// **Arithmetic, so it is tested as arithmetic.** Every case here is a function
// of the wire — no DOM to start and no story to write — and the whole value of
// `answeredAs` is the routing decision, which is invisible in a rendering.
//
// The three things worth pinning:
//
// - The press leaves the composer, so a proposal that worked says nothing. The
//   Jobs are rows on the Board and a toast over them would announce something
//   somebody is already looking at.
// - A person's own stop is not a failure. It was falling through to `refused`
//   and drawing a red notice at somebody for pressing the button on offer.
// - A decline and a fault are two drawings. One is told and one is raised, and
//   a regression here is silent — both look like "something went wrong".

import { expect, test } from "vitest";
import type { WireError } from "@armada/protocol";

import { answeredAs, watchOf } from "./proposal";
import type { Proposed } from "./proposal";

const SENT = "The board flickers every time an event lands.";

function error(over: Partial<WireError> = {}): WireError {
  return {
    code: "fleet.model.budget_exhausted",
    message: "The proposer was not called: the model budget is spent.",
    run_id: "run_8f21c0",
    fields: { budget_window: "day" },
    chain: [],
    ...over,
  };
}

/**
 * A proposal that worked says nothing at all. Every Job it named exists, the
 * Board lists them, and the press already left the surface that asked.
 */
test("a proposal that answered says nothing", () => {
  const read = answeredAs({ ok: true, jobs: [] });
  expect(read).toEqual({ outcome: null, told: null });
});

/**
 * **The claim `fleet.proposer_stopped` exists for.** `crates/fleet/src/
 * refusing.rs` declares the code apart so a client does not draw a person's own
 * press as Armada breaking, and nothing on Bridge's side matched it — so a stop
 * drew a red failure notice on whatever surface was up.
 */
test("a stop a person pressed raises nothing and tells nothing", () => {
  const answer: Proposed = {
    ok: false,
    why: "stopped",
    request: SENT,
    outcome: { ok: false, why: "refused", error: error({ code: "fleet.proposer_stopped" }) },
  };
  expect(answeredAs(answer)).toEqual({ outcome: null, told: null });
});

/**
 * No workflow resolved is told, not raised: Fleet read the request and
 * declined, which is Armada working. **It carries no `Outcome`**, so nothing
 * draws the error treatment over it.
 */
test("no workflow resolved is told and never raised", () => {
  const answer: Proposed = {
    ok: false,
    why: "unresolved",
    request: SENT,
    outcome: { ok: false, why: "no_workflow" },
  };
  const read = answeredAs(answer);
  expect(read.outcome).toBeNull();
  expect(read.told).toContain("No workflow");
  // What to do next, in the sentence. A decline nobody can act on is a dead end.
  expect(read.told).toContain("dispatch again");
});

/**
 * A fault is a failure and goes to the app's own pipeline, whole. `failing.ts`
 * is what turns the `WireError` into a code, a message and something quotable.
 */
test("a fault goes back as an outcome, carrying its wire error", () => {
  const outcome = { ok: false as const, why: "refused" as const, error: error() };
  const read = answeredAs({ ok: false, why: "faulted", request: SENT, outcome });
  expect(read.outcome).toEqual(outcome);
  expect(read.told).toBeNull();
});

/**
 * A command refused before it was sent is not a proposer refusal at all, and it
 * reads exactly like an approval refused for the same reason.
 */
test("a refusal before sending goes back as an outcome", () => {
  const answer: Proposed = {
    ok: false,
    why: "refused",
    outcome: { ok: false, why: "not_connected" },
  };
  const read = answeredAs(answer);
  expect(read.outcome).toEqual({ ok: false, why: "not_connected" });
  expect(read.told).toBeNull();
});

/** Nothing out is nothing to draw, rather than a wait with no numbers in it. */
test("no call out reads as no wait", () => {
  expect(watchOf(null, Date.parse("2026-09-30T10:00:00Z"))).toBeNull();
});

/**
 * The elapsed is the caller's subtraction, and the budget comes off the wire —
 * `ProposalInFlight` is the one thing that says either, so nothing here invents
 * a second way to say how long the call has been out.
 */
test("the wait is elapsed against Fleet's own budget", () => {
  const now = Date.parse("2026-09-30T10:05:00Z");
  const watch = watchOf(
    {
      proposal_id: "01M2D3ZF41001PROPOSAL001",
      model: "sonnet",
      since: "2026-09-30T10:03:30Z",
      budget_ms: 600_000,
      reached: "thinking",
      thinking_tokens: 1_840,
    },
    now,
  );
  expect(watch).toEqual({
    reached: "thinking",
    elapsedMs: 90_000,
    budgetMs: 600_000,
    model: "sonnet",
    thinkingTokens: 1_840,
  });
});

/**
 * A clock a few milliseconds behind Fleet's must not draw a call that has not
 * started yet.
 */
test("a call whose instant is ahead of the clock reads as no time at all", () => {
  const watch = watchOf(
    {
      proposal_id: "01M2D3ZF41001PROPOSAL001",
      model: "sonnet",
      since: "2026-09-30T10:00:01Z",
      budget_ms: 600_000,
      reached: "starting",
    },
    Date.parse("2026-09-30T10:00:00Z"),
  );
  expect(watch?.elapsedMs).toBe(0);
});

/** An instant that will not read is a wait with nothing known about it. */
test("an unreadable instant reads as no wait", () => {
  const watch = watchOf(
    {
      proposal_id: "01M2D3ZF41001PROPOSAL001",
      model: "sonnet",
      since: "not an instant",
      budget_ms: 600_000,
      reached: "starting",
    },
    Date.parse("2026-09-30T10:00:00Z"),
  );
  expect(watch).toBeNull();
});

/**
 * **A model this machine does not run is told, and never in the decline's
 * words.** It shipped as `fleet.no_workflow_fits` for half a day, whose
 * sentence tells a person to rephrase — which cannot change which models a
 * machine runs, and is advice about a workflow that was never wrong. Two
 * causes wanting opposite responses must not share a word.
 */
test("a model nothing holds is told in its own words, naming it and what is available", () => {
  const answer: Proposed = {
    ok: false,
    why: "model_unavailable",
    request: SENT,
    outcome: {
      ok: false,
      why: "refused",
      error: error({
        code: "fleet.proposer_model_not_held",
        fields: { request: SENT, model: "gpt-9", models: "haiku, sonnet, opus" },
      }),
    },
  };

  const read = answeredAs(answer);

  // Told, never raised: Armada worked, so this does not wear the error
  // treatment — the decline beside it makes the same argument.
  expect(read.outcome).toBeNull();
  expect(read.told).toContain("gpt-9");
  expect(read.told).toContain("haiku, sonnet, opus");
  // **The one sentence it must not be.**
  expect(read.told).not.toContain("Rephrase");
  expect(read.told).not.toContain("No workflow");
});

/**
 * A machine that names no model at all says so rather than drawing an empty
 * list. **Absent is not an empty set on screen** — a sentence trailing off
 * after "it runs" reads as a value that failed to load.
 */
test("a machine holding no model says that, rather than listing nothing", () => {
  const answer: Proposed = {
    ok: false,
    why: "model_unavailable",
    request: SENT,
    outcome: {
      ok: false,
      why: "refused",
      error: error({
        code: "fleet.proposer_model_not_held",
        fields: { request: SENT, model: "gpt-9" },
      }),
    },
  };

  expect(answeredAs(answer).told).toContain("names no model at all");
});
