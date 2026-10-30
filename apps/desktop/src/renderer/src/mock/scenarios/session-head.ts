// A Session's head with the facts that crowded it: a long title, a resolved model, a context figure with its breakdown
// and a state mark, the ledger open beside the conversation. A second Session rests at 72%, and a walk's `later` takes
// the first from 5% to 84%, where the tank's tone turns. The walk `session-head` plays it.

import type { ContextBreakdown, RepositorySummary, SessionRow } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import { onBoard } from "../moment";
import type { Scenario } from "../moment";
import { FakeSessionsFleet, held, hosted } from "../sessions-fleet";
import { onTimePassing } from "../time-passes";

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };

const AT = "2026-10-15T01:20:00.000Z";
const ID = "e07cfe09-5b21-4c3a-9d0e-7a6f1c2b8d44";
const HEAVY_ID = "b4a1d2c7-93e0-4f6a-8c15-2d7e9a0b3f61";
const DONE_ID = "c9e3a5f1-27d8-4b04-a6c2-5e1f8d7b0a39";
const HEAD_TITLE = "How does armada interface with GitHub (PRs, Issues, Projects)";

const WINDOW = 1_000_000;
const TOKENS = 48_400;
/** Where auto-compaction runs, as the harness reports it. */
const THRESHOLD = 850_000;
/** A Session climbing to warning, and one resting at 72%. */
const CLIMBED = 840_000;
const HEAVY = 720_000;

const breakdown = (total: number): ContextBreakdown => {
  const fixed = { system: 3_100, tools: 12_400, mcp: 8_200, memory: 4_600, skills: 1_900 };
  const messages = total - Object.values(fixed).reduce((a, b) => a + b, 0);
  const buffer = Math.min(WINDOW - THRESHOLD, WINDOW - total);
  return {
    categories: [
      { name: "System prompt", tokens: fixed.system, kind: "used" },
      { name: "System tools", tokens: fixed.tools, kind: "used" },
      { name: "MCP tools", tokens: fixed.mcp, kind: "used" },
      { name: "Memory files", tokens: fixed.memory, kind: "used" },
      { name: "Skills", tokens: fixed.skills, kind: "used" },
      { name: "Messages", tokens: messages, kind: "used" },
      { name: "Free space", tokens: WINDOW - total - buffer, kind: "free" },
      { name: "Autocompact buffer", tokens: buffer, kind: "buffer" },
    ],
    memory_files: [
      { path: "NOTES.md", tokens: 2_100 },
      { path: ".notes/MEMORY.md", tokens: 1_800 },
      { path: "apps/NOTES.md", tokens: 700 },
    ],
    mcp_tools: [
      { name: "list_jobs", server: "armada-fleet", tokens: 2_600 },
      { name: "get_job", server: "armada-fleet", tokens: 2_400 },
      { name: "query", server: "gitnexus", tokens: 1_500 },
    ],
    total_tokens: total,
    max_tokens: WINDOW,
    autocompact_threshold: THRESHOLD,
  };
};

const used = (context_tokens: number) => ({ context_tokens, context_window: WINDOW });

const records = () => [
  hosted(ID, {
    title: HEAD_TITLE,
    started_at: AT,
    last_turn_at: new Date().toISOString(),
    usage: used(TOKENS),
    context_breakdown: breakdown(TOKENS),
    attachments: [held("slot", "3"), held("branch", "docs/github-interface")],
    hosted: { turn: { state: "working" }, mode: "auto", running: true, model: "opus", resolved_model: "claude-opus-5-5" },
  }),
  hosted(HEAVY_ID, {
    title: "Cart totals rounding",
    started_at: AT,
    last_turn_at: new Date().toISOString(),
    usage: used(HEAVY),
    context_breakdown: breakdown(HEAVY),
    attachments: [held("slot", "4"), held("branch", "fix/cart-totals")],
    hosted: { turn: { state: "idle" }, mode: "auto", running: true, model: "opus", resolved_model: "claude-opus-5-5" },
  }),
  // A Session that said it is done: its state mark is `power`, beside the act that ends it.
  hosted(DONE_ID, {
    title: "Release notes draft",
    started_at: AT,
    last_turn_at: new Date().toISOString(),
    done_since: new Date().toISOString(),
    done_summary: "A rule that starts a Job now uses the workflow defined in the rule",
    usage: used(HEAVY),
    context_breakdown: breakdown(HEAVY),
    attachments: [held("slot", "2"), held("branch", "docs/release-notes")],
    hosted: { turn: { state: "idle" }, mode: "auto", running: true, model: "opus", resolved_model: "claude-opus-5-5" },
  }),
];

const threads = (): Record<string, SessionRow[]> => ({
  [ID]: [
    { kind: "message", id: "m1", at: AT, from: { kind: "you" }, text: "How does armada interface with GitHub?" },
    { kind: "tool", id: "t1", at: AT, text: "Read crates/adapters/src/forge.rs" },
    { kind: "message", id: "m2", at: AT, from: { kind: "agent" }, text: "Everything goes through the gh CLI, one adapter, behind the forge trait." },
  ],
  [DONE_ID]: [
    { kind: "message", id: "d1", at: AT, from: { kind: "you" }, text: "Draft the release notes." },
    { kind: "message", id: "d2", at: AT, from: { kind: "agent" }, text: "Drafted in docs/release-notes.md." },
  ],
});

function build(): Scenario {
  const board = onBoard({
    repositories: [ARMADA],
    picked: ARMADA.root,
  });
  const fleet = new FakeSessionsFleet(records(), threads());
  const served = fleet.scenario(board);
  return {
    ...served,
    name: "session-head",
    says: "A Session with a long title, a resolved model and its context as a tank, and a second at 72%",
    behaves: (handle) => {
      const own = served.behaves!(handle);
      // After the window has registered its own hook; the latest wins.
      queueMicrotask(() => onTimePassing(() => fleet.fillTo(ID, used(CLIMBED), breakdown(CLIMBED))));
      return own;
    },
  };
}

export const s235SessionHead: Scenario = build();
