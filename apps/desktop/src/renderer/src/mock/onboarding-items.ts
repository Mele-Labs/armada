// What the profiles the onboarding scenarios add would sync, by kind: each item with the scope it
// starts at. Mock data for `draft.account.add`.

import type { SyncItem, SyncKindWord } from "@armada/components";
import type { PlanUsage } from "@armada/protocol";

import type { Scenario } from "./moment";

const item = (name: string, scope: SyncItem["scope"], repo = "armada"): SyncItem => ({ name, scope, repo });

/** The personal login, added to a work machine. */
export const PERSONAL_ITEMS: Partial<Record<SyncKindWord, readonly SyncItem[]>> = {
  skills: [item("humanizer", "global"), item("impact-analysis", "global"), item("recipe-scaler", "private")],
  plugins: [item("code-simplifier@official", "global")],
  global_instructions: [item("AGENTS.md", "global"), item("instructions.md", "global")],
  sub_agents: [item("bridge-engineer", "repo"), item("rust-engineer", "repo")],
  commands: [item("land", "global")],
  mcp_servers: [item("gitnexus", "global"), item("linear", "private")],
  allowlist: [item("git log --oneline", "global"), item("gh pr view", "global"), item("cargo check", "repo")],
  workflows: [item("bug", "global"), item("feature", "repo")],
  memory: [item("Walks, not pictures", "repo"), item("Never generate load", "global"), item("Icons by registry name", "repo")],
};

/** The work login, added to a home machine. */
export const WORK_ITEMS: Partial<Record<SyncKindWord, readonly SyncItem[]>> = {
  skills: [item("code-review", "global", "api"), item("release-notes", "repo", "api")],
  workflows: [item("deploy", "repo", "api")],
  memory: [item("Release checklist", "repo", "api")],
};

/** The plan's windows the title bar draws beside the account chip: the 5-hour one at a normal level, the weekly one in the warning level. */
const reading = (): PlanUsage => {
  const later = (hours: number, minutes: number) => new Date(Date.now() + (hours * 60 + minutes) * 60_000 + 30_000).toISOString();
  return {
    windows: [
      { kind: "five_hour", percent_used: 34, resets_at: later(2, 14) },
      { kind: "seven_day", percent_used: 84, resets_at: later(2 * 24 + 17, 0) },
    ],
    read_at: new Date().toISOString(),
  };
};

/** A scenario's `behaves`, answering the plan's reading as well as whatever it already answered. */
export const withPlanUsage = (behaves: Scenario["behaves"]): NonNullable<Scenario["behaves"]> => (handle) => ({
  ...(behaves?.(handle) ?? {}),
  getPlanUsage: async () => ({ ok: true, value: reading() }),
  onPlanUsageChanged: () => () => {},
});
