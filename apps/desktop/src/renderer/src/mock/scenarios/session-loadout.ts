// A Session whose agent reported its Loadout: skills from the repository, Armada, Kit and a plugin, two MCP servers
// of which one failed, and four turns, the third dispatching `armada:mechanic` on sonnet, calling a GitHub MCP tool
// and a repository skill. The walk `session-loadout` plays it.

import type { LoadoutItem, LoadoutSummary, LoadoutTurn, LoadoutUse, RepositorySummary, SessionLoadout, SessionRow } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import { onBoard } from "../moment";
import type { Scenario } from "../moment";
import { FakeSessionsFleet, held, hosted } from "../sessions-fleet";

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };

const AT = "2026-10-17T13:40:00.000Z";
const ID = "a3f7c2d9-6b18-4e05-9c3a-1d8e2f4b7a60";

const item = (kind: LoadoutItem["kind"], name: string, source: LoadoutItem["source"], more: Partial<LoadoutItem> = {}): LoadoutItem => ({ kind, name, source, ...more });

const ITEMS: LoadoutItem[] = [
  item("skill", "work-issue", "repository"),
  item("skill", "armada-components", "repository"),
  item("skill", "armada-voice", "repository"),
  item("skill", "commit-message", "repository"),
  item("skill", "armada:mechanic", "armada"),
  item("skill", "armada:session-rules", "armada"),
  item("skill", "pdf", "kit"),
  item("skill", "deep-research", "kit"),
  item("skill", "productivity:memory-management", "plugin", { from: "productivity" }),
  item("skill", "code-review", "built_in"),
  item("subagent", "armada:mechanic", "armada"),
  item("subagent", "Explore", "built_in"),
  item("subagent", "general-purpose", "built_in"),
  item("mcp_server", "github", "mcp", { from: "github", state: "connected" }),
  item("mcp_server", "gitnexus", "mcp", { from: "gitnexus", state: "failed" }),
  item("mcp_server", "armada-fleet", "armada", { state: "connected" }),
  item("tool", "Read", "built_in", { state: "loaded" }),
  item("tool", "Edit", "built_in", { state: "loaded" }),
  item("tool", "Bash", "built_in", { state: "loaded" }),
  item("tool", "Grep", "built_in", { state: "loaded" }),
  item("tool", "Agent", "built_in", { state: "loaded" }),
  item("tool", "mcp__github__pull_request_read", "mcp", { from: "github", state: "loaded" }),
  item("tool", "mcp__github__search_code", "mcp", { from: "github", state: "deferred" }),
  item("tool", "mcp__armada-fleet__list_jobs", "armada", { state: "deferred" }),
  item("memory_file", "CLAUDE.md", "repository", { tokens: 2_100 }),
  item("memory_file", "docs/practices/bridge.md", "repository", { tokens: 4_800 }),
  item("memory_file", "~/.claude/CLAUDE.md", "kit", { tokens: 600 }),
  item("memory_file", "Organisation policy", "managed", { tokens: 300 }),
  item("output_style", "Explanatory", "kit"),
  item("rules", "Armada session rules", "armada"),
  item("injection", "Owner's corrections", "armada"),
  item("injection", "Managed settings", "managed"),
];

const at = (minutes: number) => new Date(Date.parse(AT) + minutes * 60_000).toISOString();
const use = (turn: string, kind: LoadoutUse["kind"], name: string, source: LoadoutUse["source"], more: Partial<LoadoutUse> = {}): LoadoutUse => ({ turn_started_at: turn, kind, name, source, ...more });

const turn = (minutes: number, used: (started: string) => LoadoutUse[]): LoadoutTurn => {
  const started = at(minutes);
  return { started_at: started, used: used(started) };
};

const TURNS: LoadoutTurn[] = [
  turn(0, (t) => [use(t, "skill", "work-issue", "repository"), use(t, "tool", "Read", "built_in"), use(t, "tool", "Grep", "built_in")]),
  turn(6, (t) => [use(t, "skill", "armada-components", "repository"), use(t, "tool", "Edit", "built_in"), use(t, "tool", "Bash", "built_in")]),
  turn(14, (t) => [
    use(t, "subagent", "armada:mechanic", "armada", { model: "sonnet" }),
    use(t, "tool", "mcp__github__pull_request_read", "mcp", { from: "github", by: "armada:mechanic" }),
    use(t, "skill", "commit-message", "repository"),
    use(t, "tool", "Bash", "built_in", { by: "armada:mechanic" }),
  ]),
  turn(22, (t) => [use(t, "skill", "armada-voice", "repository"), use(t, "tool", "Edit", "built_in")]),
];

const LOADOUT: SessionLoadout = { items: ITEMS, turns: TURNS };

const TOLD = new Set<LoadoutItem["kind"]>(["output_style", "rules", "injection"]);
const SUMMARY: LoadoutSummary = {
  knows: ITEMS.filter((one) => !TOLD.has(one.kind)).length,
  told: ITEMS.filter((one) => TOLD.has(one.kind)).length,
  latest_turn: TURNS[TURNS.length - 1]!,
};

const records = () => [
  hosted(ID, {
    title: "Why did the mechanic skip the Checks",
    started_at: AT,
    last_turn_at: new Date().toISOString(),
    usage: { context_tokens: 96_200, context_window: 1_000_000 },
    loadout_summary: SUMMARY,
    attachments: [held("slot", "2"), held("branch", "fix/mechanic-checks")],
    hosted: { turn: { state: "idle" }, mode: "auto", running: true, model: "opus", resolved_model: "claude-opus-5-5" },
  }),
];

const threads = (): Record<string, SessionRow[]> => ({
  [ID]: [
    { kind: "message", id: "m1", at: AT, from: { kind: "you" }, text: "The mechanic pushed without running the Checks. Find out why." },
    { kind: "message", id: "m2", at: at(14), from: { kind: "agent" }, text: "Sent armada:mechanic to read the pull request." },
    { kind: "message", id: "m3", at: at(22), from: { kind: "agent" }, text: "It ran on sonnet and read the pull request through GitHub's MCP server. gitnexus failed to connect, so it had no code graph." },
  ],
});

function build(): Scenario {
  const fleet = new FakeSessionsFleet(records(), threads());
  fleet.loadouts = { [ID]: LOADOUT };
  const served = fleet.scenario(onBoard({ repositories: [ARMADA], picked: ARMADA.root }));
  return { ...served, name: "session-loadout", says: "A Session's Loadout: what its agent knows, was told and used each turn, and where each came from" };
}

export const sessionLoadout: Scenario = build();
