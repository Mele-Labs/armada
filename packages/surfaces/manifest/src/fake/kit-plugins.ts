// A machine whose setup comes partly from plugins: skills, sub agents and commands under the plugin's
// name, beside the person's own and a repository's. The System sidebar's Kit entries and the walk
// `systemSidebar` play it.
//
// `plugin` on `SetupItem` is the plugin that brought the item, absent for your own.

import type { KitInventory, SetupItem, SetupKindRow } from "@armada/protocol";

const HOME = "/Users/user/.claude";
const REPOSITORY = "/Users/user/develop/armada";
const CACHE = `${HOME}/plugins/cache`;
const SERVERS = "/Users/user/.claude.json";

const read = (kind: SetupKindRow["kind"], items: SetupItem[]): SetupKindRow => ({ kind, read: { what: "read", items, unreadable: [] } });

const WIDE: { harness: string; home: string; present: boolean; kinds: SetupKindRow[] } = {
  harness: "Claude Code",
  home: HOME,
  present: true,
  kinds: [
    read("skills", [
      { name: "humanizer", says: "Rewrite AI-sounding text so it reads like the writer without changing what it says.", source: `${HOME}/skills/humanizer` },
      { name: "impact-analysis", says: "Use when the user wants to know what will break if they change something.", source: `${HOME}/skills/impact-analysis` },
      { name: "frontend-design", says: "Guidance for distinctive, intentional visual design when building new UI or reshaping an existing one.", source: `${CACHE}/frontend-design/1.2.0/skills/frontend-design`, plugin: "frontend-design" },
      { name: "skill-creator", says: "Create new skills, modify and improve existing skills, and measure skill performance.", source: `${CACHE}/skill-creator/0.9.4/skills/skill-creator`, plugin: "skill-creator" },
      { name: "reading-a-brief", says: "Work from a brief someone handed you: find your place by search and line ranges, stay inside the paths it names.", source: `${CACHE}/armada/1.0.0/skills/reading-a-brief`, plugin: "armada" },
      { name: "verify-before-done", says: "Prove a change works before saying it is done.", source: `${CACHE}/armada/1.0.0/skills/verify-before-done`, plugin: "armada" },
    ]),
    read("plugins", [
      { name: "code-simplifier@official", says: "version 1.0.0", source: `${CACHE}/code-simplifier/1.0.0` },
      { name: "frontend-design@official", says: "version 1.2.0", source: `${CACHE}/frontend-design/1.2.0` },
      { name: "skill-creator@official", says: "version 0.9.4", source: `${CACHE}/skill-creator/0.9.4` },
      { name: "armada@local", says: "version 1.0.0", source: `${CACHE}/armada/1.0.0` },
    ]),
    read("agent_file", [
      { name: "CLAUDE.md", says: "21 lines, opening # Global Instructions", source: `${HOME}/CLAUDE.md` },
    ]),
    read("output_styles", [
      { name: "bluf", says: "Bottom line first, then only what the reader needs to act on it.", source: `${HOME}/output-styles/bluf.md`, style: { from: "person", applied: "active" } },
      { name: "Armada", says: "Answers the way Armada's own jobs do: facts, no filler.", source: `${CACHE}/armada/1.0.0/output-styles/Armada.md`, style: { from: "plugin", applied: "available", plugin: "armada" }, plugin: "armada" },
      { name: "terse-review", says: "Short findings for reading a diff, one line each.", source: `${REPOSITORY}/.claude/output-styles/terse-review.md`, style: { from: "repository", applied: "available" } },
      { name: "Explanatory", says: "Explains the choices it makes as it works.", source: "built in to Claude Code", style: { from: "built_in", applied: "available" } },
      { name: "Armada job rules", says: "Armada's rules for the jobs Bridge hosts, applied on top of the active style.", source: "Armada", style: { from: "armada", applied: "always" } },
    ]),
    read("sub_agents", [
      { name: "reviewer", says: "Reads a diff against the brief it was written for and reports what does not hold.", source: `${HOME}/agents/reviewer.md` },
      { name: "code-simplifier", says: "Simplifies and refines code for clarity, consistency, and maintainability while preserving all functionality.", source: `${CACHE}/code-simplifier/1.0.0/agents/code-simplifier.md`, plugin: "code-simplifier" },
      { name: "implementer", says: "Builds a change whose shape is still being decided: a feature, a design choice, a change that crosses a module boundary.", source: `${CACHE}/armada/1.0.0/agents/implementer.md`, plugin: "armada" },
      { name: "explorer", says: "Finds where something is and how it works, and reports the answer with file and line.", source: `${CACHE}/armada/1.0.0/agents/explorer.md`, plugin: "armada" },
    ]),
    read("commands", [
      { name: "standup", says: "Write the day's notes from the branches touched.", source: `${HOME}/commands/standup.md` },
      { name: "land", says: "Merge the pull request this job opened once ci is green.", source: `${CACHE}/armada/1.0.0/commands/land.md`, plugin: "armada" },
      { name: "update-armada", says: "Move the Fleet, Bridge and the plugin onto the latest main in one go.", source: `${CACHE}/armada/1.0.0/commands/update-armada.md`, plugin: "armada" },
      { name: "run-evals", says: "Run a skill's evals and report the pass rate.", source: `${CACHE}/skill-creator/0.9.4/commands/run-evals.md`, plugin: "skill-creator" },
    ]),
    read("mcp_servers", [
      { name: "gitnexus", says: "gitnexus-mcp", source: SERVERS },
      { name: "linear", says: "mcp.linear.app", source: SERVERS },
    ]),
    read("allowlist", [
      { name: "cargo check", source: "written by hand" },
      { name: "pnpm exec tsc", source: "always allow" },
      { name: "gh pr view", source: "retro item 7" },
    ]),
    { kind: "models", read: { what: "not_read", why: "which models a Job may use is resolved against a Manifest, and that is #41" } },
  ],
};

/** The default inventory with plugins, a repository's own items and two instruction files. */
export const PLUGIN_KIT_INVENTORY: KitInventory = WIDE;
