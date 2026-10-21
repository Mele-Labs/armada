// A Session's Loadout with a realistic mix, for the stories of its mark and its sheet.

import type { LoadoutItem, LoadoutTurn, SessionLoadout } from "@armada/protocol";

import type { LoadoutItemRead } from "./LoadoutItemPanel";
import type { McpServerRow } from "./LoadoutMcp";

const skill = (name: string, source: LoadoutItem["source"], from?: string): LoadoutItem => ({ kind: "skill", name, source, ...(from === undefined ? {} : { from }) });
const tool = (name: string, source: LoadoutItem["source"], state: LoadoutItem["state"], from?: string): LoadoutItem => ({ kind: "tool", name, source, ...(state === undefined ? {} : { state }), ...(from === undefined ? {} : { from }) });

export const ITEMS: LoadoutItem[] = [
  skill("work-issue", "repository"),
  skill("armada-components", "repository"),
  skill("armada-voice", "repository"),
  skill("armada:mechanic", "armada"),
  skill("productivity:memory-management", "plugin", "productivity"),
  skill("commit-message", "kit"),
  skill("code-review", "built_in"),
  { kind: "subagent", name: "armada:mechanic", source: "armada" },
  { kind: "subagent", name: "general-purpose", source: "built_in" },
  { kind: "mcp_server", name: "forge", source: "mcp", from: "forge", state: "connected" },
  { kind: "mcp_server", name: "gitnexus", source: "mcp", from: "gitnexus", state: "failed" },
  { kind: "mcp_server", name: "armada-fleet", source: "armada", state: "pending" },
  tool("Read", "built_in", "loaded"),
  tool("Edit", "built_in", "loaded"),
  tool("Bash", "built_in", "loaded"),
  tool("mcp__forge__pull_request_read", "mcp", "deferred", "forge"),
  tool("mcp__forge__search_code", "mcp", "deferred", "forge"),
  { kind: "memory_file", name: "AGENTS.md", source: "repository", tokens: 2_100 },
  { kind: "memory_file", name: "~/.harness/AGENTS.md", source: "kit", tokens: 600 },
  { kind: "output_style", name: "Explanatory", source: "kit" },
  { kind: "rules", name: "Armada job rules", source: "armada" },
  { kind: "injection", name: "Owner's corrections", source: "managed" },
];

export const TURNS: LoadoutTurn[] = [
  {
    started_at: "2026-10-17T14:02:00.000Z",
    used: [
      { turn_started_at: "2026-10-17T14:02:00.000Z", kind: "skill", name: "work-issue", source: "repository" },
      { turn_started_at: "2026-10-17T14:02:00.000Z", kind: "tool", name: "Read", source: "built_in" },
    ],
  },
  {
    started_at: "2026-10-17T14:09:00.000Z",
    used: [
      { turn_started_at: "2026-10-17T14:09:00.000Z", kind: "subagent", name: "armada:mechanic", source: "armada", model: "sonnet" },
      { turn_started_at: "2026-10-17T14:09:00.000Z", kind: "tool", name: "mcp__forge__pull_request_read", source: "mcp", from: "forge", by: "armada:mechanic", count: 40 },
      { turn_started_at: "2026-10-17T14:09:00.000Z", kind: "skill", name: "armada-components", source: "repository" },
    ],
  },
];

export const LOADOUT: SessionLoadout = { items: ITEMS, turns: TURNS };

export const SUMMARY = { knows: 19, told: 3, latest_turn: TURNS[TURNS.length - 1]! };

/** What two items say, by `loadoutKeyOf`: a repository skill, and the session rules cut at the cap. */
export const BODIES: Record<string, LoadoutItemRead> = {
  "skill:repository::work-issue": {
    state: "read",
    path: ".harness/skills/work-issue/SKILL.md",
    format: "markdown",
    body: "---\nname: work-issue\ndescription: How to work one issue\n---\n\n# Work one issue\n\nCut a worktree, plan, implement, check, land.\n",
  },
  "rules:armada::Armada job rules": {
    state: "read",
    format: "text",
    body: "You are a Session of Armada.\nRead the issue before the code.\n",
    truncated: true,
  },
};

/** A hosted Job's servers as its CLI reports them, one in each state: the owner's sketch of 21 Oct 2026. */
export const MCP_SERVERS: McpServerRow[] = [
  { name: "armada-fleet", state: "connected", tools: 71 },
  { name: "chrome-devtools", state: "failed", error: "timed out after 30s", tools: 0 },
  { name: "Atlassian Rovo (2)", state: "connecting", tools: 0 },
  { name: "mailroom", state: "needs_sign_in", tools: 0 },
  { name: "ledger", state: "off", tools: 0 },
];
