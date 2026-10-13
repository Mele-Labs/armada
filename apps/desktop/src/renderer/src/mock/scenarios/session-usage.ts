// Context fill and plan usage. Three live Sessions at three context levels, and the title bar holding the
// plan's limits at 5h 34% and wk 61%. A walk's `later` moves the limits to 5h 86% and wk 97%, so the
// warning and danger colours can be seen there too. One Session and one running Job are held for the plan
// limit, resuming 2h 14m from now. The walk `session-usage` plays it.

import type { RepositorySummary } from "@armada/protocol";
import type { PlanUsage } from "@armada/components";
import { job, repository } from "@armada/screens/src/fixtures/build/base";

import { onBoard } from "../moment";
import type { Scenario } from "../moment";
import { mockPlanUsage } from "../plan-usage";
import { limitHeld } from "@armada/screens/src/limit-held";
import { FakeSessionsFleet, hosted } from "../sessions-fleet";
import { onTimePassing } from "../time-passes";

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };

const now = () => new Date().toISOString();

const used = (context_tokens: number, context_window: number) => ({ context_tokens, context_window });

const records = () => [
  hosted("01CTXLOWAAAAAAAAAAAAAAAAAA", { title: "Release notes script", last_turn_at: now(), usage: used(53_840, 1_000_000) }),
  hosted("01CTXMIDBBBBBBBBBBBBBBBBBB", { title: "Store migration spike", last_turn_at: now(), usage: used(142_000, 200_000) }),
  hosted("01CTXLIMDDDDDDDDDDDDDDDDDD", { title: "Invoice PDF layout", last_turn_at: now(), usage: used(192_000, 200_000) }),
  hosted("01CTXHIGCCCCCCCCCCCCCCCCCC", { title: "Cart totals rounding", last_turn_at: now(), usage: used(188_000, 200_000) }),
];

/** This long from now, as the mod would send it: the card's clock time and its countdown agree. */
const fromNow = (hours: number, minutes: number): string => new Date(Date.now() + (hours * 60 + minutes) * 60_000 + 30_000).toISOString();

const roomy = (): PlanUsage[] => [
  { kind: "five_hour", percentUsed: 34.2, resetsAt: fromNow(2, 14) },
  { kind: "seven_day", percentUsed: 61.4, resetsAt: fromNow(2 * 24 + 17, 0) },
];

const full = (): PlanUsage[] => [
  { kind: "five_hour", percentUsed: 86.1, resetsAt: fromNow(2, 14) },
  { kind: "seven_day", percentUsed: 97.3, resetsAt: fromNow(2 * 24 + 17, 0) },
];

function build(): Scenario {
  const board = onBoard([job("running", { id: "01JOBLIMITED", handle: "61-the-invoice-totals", title: "The invoice totals", branch: "fix/invoice-totals", owner_manifest_id: "armada" })], {
    repositories: [ARMADA],
    picked: ARMADA.root,
  });
  const served = new FakeSessionsFleet(records(), {}).scenario(board);
  return {
    ...served,
    name: "session-usage",
    says: "Three live Sessions at three context levels, and the title bar holding the plan's 5-hour and weekly limits",
    behaves: (handle) => {
      const fleet = new FakeSessionsFleet(records(), {}).scenario(board).behaves!(handle);
      // After the window has mounted and registered its own hook, and the latest wins.
      queueMicrotask(() => {
        mockPlanUsage.set(roomy());
        limitHeld.set(new Map([["01CTXLIMDDDDDDDDDDDDDDDDDD", fromNow(2, 14)], ["01JOBLIMITED", fromNow(2, 14)]]));
        onTimePassing(() => mockPlanUsage.set(full()));
      });
      return fleet;
    },
  };
}

export const s234SessionUsage: Scenario = build();
