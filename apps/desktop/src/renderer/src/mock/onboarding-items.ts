// What the profiles the onboarding scenarios add would sync, by kind: each item with the scope it
// starts at. Mock data for `draft.account.add`.

import type { SyncItem, SyncKindWord } from "@armada/components";

const item = (name: string, scope: SyncItem["scope"], repo = "armada"): SyncItem => ({ name, scope, repo });

/** The personal login, added to a work machine. */
export const PERSONAL_ITEMS: Partial<Record<SyncKindWord, readonly SyncItem[]>> = {
  skills: [item("humanizer", "global"), item("impact-analysis", "global"), item("recipe-scaler", "private")],
  plugins: [item("code-simplifier@official", "global")],
  agent_file: [item("AGENTS.md", "global")],
  global_instructions: [item("CLAUDE.md", "global")],
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
