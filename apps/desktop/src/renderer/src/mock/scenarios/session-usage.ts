// Context fill and plan usage. Three live Sessions at three context levels, and the title bar holding the
// plan's limits at 5h 34% and wk 61%. A walk's `later` moves the limits to 5h 86% and wk 97%, so the
// warning and danger colours can be seen there too. One Session and one running Job are held for the plan
// limit, resuming 2h 14m from now. The walk `session-usage` plays it.

import type { ContextBreakdown, PlanUsage, RepositorySummary } from "@armada/protocol";
import { job, repository } from "@armada/screens/src/fixtures/build/base";

import { onBoard } from "../moment";
import type { Scenario } from "../moment";
import { FakeSessionsFleet, hosted } from "../sessions-fleet";
import { onTimePassing } from "../time-passes";

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };

const now = () => new Date().toISOString();

const used = (context_tokens: number, context_window: number) => ({ context_tokens, context_window });

/** This long from now, as the mod would send it: the card's clock time and its countdown agree. */
const fromNow = (hours: number, minutes: number): string => new Date(Date.now() + (hours * 60 + minutes) * 60_000 + 30_000).toISOString();

const records = () => [
  hosted("01CTXLOWAAAAAAAAAAAAAAAAAA", { title: "Release notes script", last_turn_at: now(), usage: used(53_840, 1_000_000), context_breakdown: breakdownOf(53_840, 1_000_000) }),
  hosted("01CTXMIDBBBBBBBBBBBBBBBBBB", { title: "Store migration spike", last_turn_at: now(), usage: used(142_000, 200_000), context_breakdown: breakdownOf(142_000, 200_000) }),
  hosted("01CTXLIMDDDDDDDDDDDDDDDDDD", { title: "Invoice PDF layout", last_turn_at: now(), usage: used(192_000, 200_000), context_breakdown: breakdownOf(192_000, 200_000), held_until: fromNow(2, 14) }),
  hosted("01CTXHIGCCCCCCCCCCCCCCCCCC", { title: "Cart totals rounding", last_turn_at: now(), usage: used(188_000, 200_000), context_breakdown: breakdownOf(188_000, 200_000) }),
];

/**
 * What fills a window of `max` tokens holding `total`: the fixed parts every Session carries, the rest
 * Messages, the autocompact buffer where it fits, and the room that is left. The categories sum to `max`.
 */
function breakdownOf(total: number, max: number): ContextBreakdown {
  const fixed = { system: 3_100, tools: 12_400, mcp: 8_200, memory: 4_600, skills: 1_900 };
  const messages = total - Object.values(fixed).reduce((a, b) => a + b, 0);
  const buffer = Math.min(33_000, max - total);
  return {
    categories: [
      { name: "System prompt", tokens: fixed.system, kind: "used" },
      { name: "System tools", tokens: fixed.tools, kind: "used" },
      { name: "MCP tools", tokens: fixed.mcp, kind: "used" },
      { name: "Memory files", tokens: fixed.memory, kind: "used" },
      { name: "Skills", tokens: fixed.skills, kind: "used" },
      { name: "Messages", tokens: messages, kind: "used" },
      { name: "Free space", tokens: max - total - buffer, kind: "free" },
      { name: "Autocompact buffer", tokens: buffer, kind: "buffer" },
    ],
    memory_files: [
      { path: "CLAUDE.md", tokens: 2_100 },
      { path: ".claude/MEMORY.md", tokens: 1_800 },
      { path: "apps/CLAUDE.md", tokens: 700 },
    ],
    mcp_tools: [
      { name: "list_jobs", server: "armada-fleet", tokens: 2_600 },
      { name: "get_job", server: "armada-fleet", tokens: 2_400 },
      { name: "query", server: "gitnexus", tokens: 1_500 },
      { name: "impact", server: "gitnexus", tokens: 800 },
      { name: "query-docs", server: "context7", tokens: 900 },
    ],
    total_tokens: total,
    max_tokens: max,
  };
}

const roomy = (): PlanUsage => ({
  windows: [
    { kind: "five_hour", percent_used: 34, resets_at: fromNow(2, 14) },
    { kind: "seven_day", percent_used: 61, resets_at: fromNow(2 * 24 + 17, 0) },
  ],
  read_at: now(),
});

const full = (): PlanUsage => ({
  windows: [
    { kind: "five_hour", percent_used: 86, resets_at: fromNow(2, 14) },
    { kind: "seven_day", percent_used: 97, resets_at: fromNow(2 * 24 + 17, 0) },
  ],
  read_at: now(),
});

function build(): Scenario {
  const held = { by: "fleet", at: now(), resuming: false, until: fromNow(2, 14) };
  const board = onBoard([job("running", { id: "01JOBLIMITED", handle: "61-the-invoice-totals", title: "The invoice totals", branch: "fix/invoice-totals", owner_manifest_id: "armada", paused: held })], {
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
      // What Fleet answers for the plan, and tells again as a session reports a newer reading.
      let reading = roomy();
      const heard = new Set<(usage: PlanUsage) => void>();
      // After the window has registered its own hook, and the latest wins: a walk's `later` is a newer reading.
      queueMicrotask(() =>
        onTimePassing(() => {
          reading = full();
          heard.forEach((on) => on(reading));
        }),
      );
      return {
        ...fleet,
        getPlanUsage: async () => ({ ok: true, value: reading }),
        onPlanUsageChanged: (on) => {
          heard.add(on);
          return () => void heard.delete(on);
        },
      };
    },
  };
}

export const s234SessionUsage: Scenario = build();
