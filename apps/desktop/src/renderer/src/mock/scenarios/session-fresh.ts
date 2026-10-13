// Which model a Session runs on, and a start-fresh mark once one is big or old. Three live Sessions: one
// small and new on the default model, one old on a picked model, one big on the default. The walk
// `session-fresh` plays it.

import type { RepositorySummary, SessionRow } from "@armada/protocol";
import { job, repository } from "@armada/screens/src/fixtures/build/base";

import { onBoard } from "../moment";
import type { Scenario } from "../moment";
import { HANDOFF_ASK } from "../../session-handoff";
import { FakeSessionsFleet, hosted } from "../sessions-fleet";

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };

const AT = "2026-10-07T13:48:02.000Z";
const SMALL = "01FRESH1AAAAAAAAAAAAAAAAAA";
const OLD = "01FRESH2AAAAAAAAAAAAAAAAAA";
const BIG = "01FRESH3AAAAAAAAAAAAAAAAAA";

const HANDOFF = "Goal: cart totals match the invoice to the cent. The sum is taken before rounding in totals.rs, and the test for it passes. Branch fix/cart-totals, pull request 412 open. Next: the same rounding in the refund path.";

/** This long ago, as the record's `started_at`. */
const ago = (hours: number): string => new Date(Date.now() - hours * 3_600_000).toISOString();
const now = () => new Date().toISOString();
const used = (context_tokens: number) => ({ context_tokens, context_window: 1_000_000 });

const records = () => [
  hosted(SMALL, {
    title: "Release notes script",
    started_at: ago(2),
    last_turn_at: now(),
    usage: used(53_840),
    hosted: { turn: { state: "idle" }, mode: "auto", running: true, resolved_model: "vendor-opus-5-5" },
  }),
  hosted(OLD, {
    title: "Invoice PDF layout",
    started_at: ago(52),
    last_turn_at: now(),
    usage: used(88_000),
    hosted: { turn: { state: "idle" }, mode: "auto", running: true, model: "sonnet", resolved_model: "vendor-sonnet-5-5" },
  }),
  hosted(BIG, {
    title: "Cart totals rounding",
    started_at: ago(3),
    last_turn_at: now(),
    usage: used(612_000),
    hosted: { turn: { state: "idle" }, mode: "auto", running: true, resolved_model: "vendor-opus-5-5" },
  }),
];

const threads = (): Record<string, SessionRow[]> => ({
  [BIG]: [
    { kind: "message", id: "m1", at: AT, from: { kind: "you" }, text: "Cart totals are off by a cent on some orders. Find out why." },
    { kind: "tool", id: "t1", at: AT, text: "Read crates/cart/src/totals.rs" },
    { kind: "message", id: "m2", at: AT, from: { kind: "agent" }, text: "Each line is rounded before the sum. Summing first, then rounding once, matches the invoice." },
  ],
  [OLD]: [
    { kind: "message", id: "m1", at: AT, from: { kind: "you" }, text: "The invoice PDF clips its last row." },
    { kind: "message", id: "m2", at: AT, from: { kind: "agent" }, text: "The footer is positioned from the page bottom, over the table. I moved the table up by the footer's height." },
  ],
});

/** A Fleet of its own for each window, so a walk told twice starts from the same Sessions. */
function fleet(): FakeSessionsFleet {
  const made = new FakeSessionsFleet(records(), threads());
  made.startsNow = true;
  made.keepsRows = true;
  // The old Session writes its own handoff when asked for one.
  made.onSend = (send) => {
    if (send.text !== HANDOFF_ASK) return;
    void made.agentTurn(send.session_id, [
      { kind: "message", id: "h1", at: AT, from: { kind: "agent" }, text: HANDOFF },
    ]);
  };
  return made;
}

function build(): Scenario {
  const board = onBoard([job("running", { id: "01JOBFRESH", handle: "61-the-invoice-totals", title: "The invoice totals", branch: "fix/invoice-totals", owner_manifest_id: "armada" })], {
    repositories: [ARMADA],
    picked: ARMADA.root,
  });
  const served = fleet().scenario(board);
  return {
    ...served,
    name: "session-fresh",
    says: "Three live Sessions: the model each runs on, and a fresh-start mark on the ones that are big or old",
    behaves: (handle) => fleet().scenario(board).behaves!(handle),
  };
}

export const s240SessionFresh: Scenario = build();
