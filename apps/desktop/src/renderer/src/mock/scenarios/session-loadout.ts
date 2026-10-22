// A hosted Session whose agent reported its Loadout: skills from the repository, Armada, Kit and a plugin, MCP servers
// read live from its process with one in each state the CLI reports, and four turns, the third dispatching `armada:mechanic` on sonnet, calling a forge MCP tool
// and a repository skill. What a row opens is served for a repository skill, an Armada agent file, AGENTS.md and the
// session rules, and a built in tool says why it has no file. chrome-devtools timed out and reconnects, mailroom
// signs in on a connector page that sends nothing back, and forge turns off and on. Kit's inventory holds the two own
// skills and the productivity plugin's, so they offer Edit; an own skill is removed to a pretend Trash and restored from
// it, and the plugin uninstalled. The walk `session-loadout` plays it.

import type { KitInventory, LoadoutItem, LoadoutSummary, LoadoutTurn, LoadoutUse, RepositorySummary, SessionLoadout, SessionMcpServer, SessionRow } from "@armada/protocol";
import { loadoutKeyOf } from "@armada/components";
import { kitPieces } from "@armada/manifest/fake";
import type { LoadoutItemBody } from "@armada/screens/src/draft/sessions";
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
  item("mcp_server", "forge", "mcp", { from: "forge", state: "connected" }),
  item("mcp_server", "chrome-devtools", "mcp", { from: "chrome-devtools", state: "failed" }),
  item("mcp_server", "armada-fleet", "armada", { state: "connected" }),
  item("tool", "Read", "built_in", { state: "loaded" }),
  item("tool", "Edit", "built_in", { state: "loaded" }),
  item("tool", "Bash", "built_in", { state: "loaded" }),
  item("tool", "Grep", "built_in", { state: "loaded" }),
  item("tool", "Agent", "built_in", { state: "loaded" }),
  item("tool", "mcp__forge__pull_request_read", "mcp", { from: "forge", state: "loaded" }),
  item("tool", "mcp__forge__search_code", "mcp", { from: "forge", state: "deferred" }),
  item("tool", "mcp__armada-fleet__list_jobs", "armada", { state: "deferred" }),
  item("memory_file", "AGENTS.md", "repository", { tokens: 2_100 }),
  item("memory_file", "docs/practices/bridge.md", "repository", { tokens: 4_800 }),
  item("memory_file", "~/.harness/AGENTS.md", "kit", { tokens: 600 }),
  item("memory_file", "Organisation policy", "managed", { tokens: 300 }),
  item("output_style", "Explanatory", "kit"),
  item("rules", "Armada job rules", "armada"),
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
    use(t, "tool", "mcp__forge__pull_request_read", "mcp", { from: "forge", by: "armada:mechanic", count: 40 }),
    use(t, "skill", "commit-message", "repository"),
    use(t, "tool", "Bash", "built_in", { by: "armada:mechanic" }),
  ]),
  turn(22, (t) => [
    use(t, "skill", "armada-voice", "repository"),
    use(t, "tool", "Edit", "built_in"),
    use(t, "tool", "mcp__mailroom_docs__create_inspiration_document", "mcp", { from: "mailroom_docs" }),
    use(t, "tool", "mcp__browser-devtools__resize_page", "mcp", { from: "browser-devtools" }),
  ]),
];

const LOADOUT: SessionLoadout = { items: ITEMS, turns: TURNS };

const WORK_ISSUE = `---
name: work-issue
description: How to work one issue: worktree, plan, implement, self-check, commit, land. Load before starting any issue, and before dispatching agents at several.
---

# Working one issue

**This is the \`bug\` workflow, run by hand.** A Session works in its own leased slot, runs \`armada check\` when it needs to, and goes up as a pull request.

## The steps

1. **Cut a worktree on a branch** before the first edit. Never edit the checkout at \`main\`.
2. **Plan** against the issue's own acceptance test, written before the code.
3. **Implement**, asking the code graph what a change breaks before the edit.
4. **Self-check**: \`cargo xtask verify-foundations\`, read as a delta against \`main\`.
5. **Commit** with \`commit-message\`, and push.
6. **Land** it as a pull request merged on a green \`ci\`.

| Step | Skill |
|---|---|
| Worktree | \`agent-worktrees\` |
| Commit | \`commit-message\` |
| Land | \`land\` |
`;

const MECHANIC = `---
name: mechanic
description: Fixes a Check that failed on a Session's branch, and pushes the fix. Dispatched by Armada when a Check fails.
model: sonnet
tools: Read, Edit, Bash, Grep, mcp__forge__pull_request_read
---

You are Armada's mechanic. A Check failed on the branch you were given.

1. Read the Check's log and the pull request.
2. Find the smallest change that makes the Check pass.
3. Run the Check again with \`armada check <name>\` before you push.
4. Push to the same branch. Never open a second pull request.

Report what failed, what you changed and the Check's result.
`;

const AGENTS_MD = `# Armada

A macOS app that dispatches coding agents against git repositories and verifies their work before advancing them. Rust daemon, Electron app, one repo.

- **Read \`cargo xtask verify-foundations\` as a delta against \`main\`, not as a colour.**
- **The acceptance test is a milestone's own claim**, written before the code.
- **Never edit the checkout at \`main\`.** Cut a worktree on a branch before the first edit.

| Looking for | Go to |
|---|---|
| What Armada is for | \`docs/scope.md\` |
| A binding rule | \`docs/contracts/\` |
| Rust, Bridge, or the seam between them | \`docs/practices/\` |
`;

const RULES = `You work in a session Armada hosts. Armada's Bridge shows the person what each session needs from them and how far its work has got, and it reads that from the tools below and from nothing else. These rules cover how needs and progress are surfaced. The person's own output style still sets tone, length and format. Where it conflicts with these rules on surfacing, these rules win.

Needs. Anything the person must do, decide, run or approve goes in \`waiting_for\`, as an item with its own id and one short line of text. A restart they must run is an item with \`act\` of \`run\`. A need is never only a row in a table, a list or a sentence in your reply.

Progress. Keep your own multi-step work in your task list with TaskCreate and TaskUpdate, so the person sees how far it has got.
`;

/** What a row opens, by `loadoutKeyOf`, as `read_loadout_item` answers. Anything else is not found. */
const BODIES: Record<string, LoadoutItemBody> = {
  [loadoutKeyOf({ kind: "skill", name: "work-issue", source: "repository" })]: { what: "read", path: ".harness/skills/work-issue/SKILL.md", format: "markdown", body: WORK_ISSUE },
  [loadoutKeyOf({ kind: "subagent", name: "armada:mechanic", source: "armada" })]: { what: "read", path: "agents/mechanic.md", format: "markdown", body: MECHANIC },
  [loadoutKeyOf({ kind: "memory_file", name: "AGENTS.md", source: "repository" })]: { what: "read", path: "AGENTS.md", format: "markdown", body: AGENTS_MD },
  [loadoutKeyOf({ kind: "rules", name: "Armada job rules", source: "armada" })]: { what: "read", format: "text", body: RULES },
  [loadoutKeyOf({ kind: "tool", name: "Read", source: "built_in" })]: { what: "not_read", why: "Read is built into the agent. It has no file to read." },
};

/** The servers as the Session's process reports them now, one in each state. */
const MCP: SessionMcpServer[] = [
  { name: "armada-fleet", state: "connected", scope: "user", tools: 71 },
  { name: "forge", state: "connected", scope: "project", tools: 2 },
  { name: "chrome-devtools", state: "failed", error: "timed out after 30s", scope: "project", tools: 0 },
  { name: "Atlassian Rovo (2)", state: "connecting", scope: "dynamic", tools: 0 },
  { name: "mailroom", state: "needs_sign_in", scope: "dynamic", tools: 0 },
  { name: "ledger", state: "off", scope: "user", tools: 0 },
];

const HOME = "/Users/user/.harness";
const PRODUCTIVITY = `${HOME}/plugins/cache/productivity/1.0.0`;

/** Kit's inventory as the Loadout matches it: the two own skills, and the productivity plugin with the two it brings. */
const KIT: KitInventory = {
  harness: "Harness",
  home: HOME,
  present: true,
  kinds: [
    {
      kind: "skills",
      read: {
        what: "read",
        items: [
          { name: "pdf", says: "Read and write PDF files", source: `${HOME}/skills/pdf` },
          { name: "deep-research", says: "Research a question across many sources", source: `${HOME}/skills/deep-research` },
          { name: "memory-management", says: "Keep notes between sessions", source: `${PRODUCTIVITY}/skills/memory-management`, plugin: "productivity" },
          { name: "summarise", says: "Summarise a long thread", source: `${PRODUCTIVITY}/skills/summarise`, plugin: "productivity" },
        ],
        unreadable: [],
      },
    },
    {
      kind: "plugins",
      read: { what: "read", items: [{ name: "productivity@toolshed", says: "version 1.0.0", source: PRODUCTIVITY, uninstall: "harness plugin uninstall productivity@toolshed" }], unreadable: [] },
    },
  ],
};

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
    hosted: { turn: { state: "idle" }, mode: "auto", running: true, model: "opus", resolved_model: "vendor-opus-5-5" },
  }),
];

const threads = (): Record<string, SessionRow[]> => ({
  [ID]: [
    { kind: "message", id: "m1", at: AT, from: { kind: "you" }, text: "The mechanic pushed without running the Checks. Find out why." },
    { kind: "message", id: "m2", at: at(14), from: { kind: "agent" }, text: "Sent armada:mechanic to read the pull request." },
    { kind: "message", id: "m3", at: at(22), from: { kind: "agent" }, text: "It ran on sonnet and read the pull request through the forge MCP server. chrome-devtools failed to connect, so it could not open the page." },
  ],
});

function build(): Scenario {
  const fleet = new FakeSessionsFleet(records(), threads());
  fleet.loadouts = { [ID]: LOADOUT };
  fleet.loadoutItems = { [ID]: BODIES };
  fleet.mcpServers = { [ID]: MCP };
  fleet.mcpTools = { "chrome-devtools": 26, mailroom: 12, forge: 2, ledger: 40 };
  fleet.mcpSignIns = { mailroom: { url: "https://mailroom.example/settings/connectors", callback_expected: false } };
  const served = fleet.scenario(onBoard({ repositories: [ARMADA], picked: ARMADA.root }));
  const behaves: Scenario["behaves"] = (handle) => {
    let inventory = KIT;
    const pieces = kitPieces({}, { get: () => inventory, set: (next) => void (inventory = next) });
    return {
      ...served.behaves?.(handle),
      readKitInventory: async () => ({ ok: true, setup: inventory }),
      removeKitPiece: pieces.removeKitPiece,
      restoreKitPiece: pieces.restoreKitPiece,
    };
  };
  return { ...served, behaves, name: "session-loadout", says: "A Session's Loadout: what its agent knows, was told and used each turn, where each came from, its MCP servers live, and Kit's skills edited, removed and restored" };
}

export const sessionLoadout: Scenario = build();
