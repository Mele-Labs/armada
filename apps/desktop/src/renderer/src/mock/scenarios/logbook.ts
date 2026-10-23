// The Logbook with something in every part: two repositories' entries, Habits and This Mac, a tripped entry, a pinned
// one, a candidate and two faded ones. `docs/concepts/logbook.md`. Mock only: no Fleet route serves the Logbook yet.
// The walk `logbook` plays over it.

import type { LogbookEntry, LogbookEvidence, RepositorySummary } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import { holding } from "../holding";
import type { Scenario } from "../moment";

const ARMADA = repository().root;
const LEDGER: RepositorySummary = {
  root: "/Users/user/ledger",
  records_root: "/Users/user/Library/Application Support/Armada/records/ledger",
};

const HOUR = 3_600_000;
const ago = (hours: number) => new Date(Date.now() - hours * HOUR).toISOString();

const row = (job_id: string, job_title: string, cite: string, says: string, hours: number): LogbookEvidence => ({ job_id, job_title, cite, says, at: ago(hours) });

/** An entry with the parts most of them share; each one below says what is its own. */
function entry(id: string, fields: Partial<LogbookEntry> & Pick<LogbookEntry, "section" | "kind" | "text">): LogbookEntry {
  return {
    id,
    evidence: [],
    confirmed_by: 2,
    first_seen: ago(24 * 9),
    last_confirmed: ago(30),
    last_served: ago(2),
    state: "standing",
    pinned: false,
    edited: false,
    history: [],
    ...fields,
  };
}

const ENTRIES: LogbookEntry[] = [
  entry("lb-vitest", {
    section: "repo",
    repository: ARMADA,
    kind: "command",
    text: "Run `pnpm exec vitest run <touched files>`. Never the whole desktop suite.",
    paths: ["apps/desktop/", "packages/"],
    confirmed_by: 7,
    first_seen: ago(24 * 14),
    last_confirmed: ago(5),
    evidence: [
      row("e2b1c4", "Keep drafts across leaving the Sessions page", "turn 41, Bash", "`pnpm test` in apps/desktop ran 31 minutes and was stopped by hand", 24 * 14),
      row("71f09a", "Walk window keeps two rows", "turn 18, Bash", "`vitest run walk-window.test.tsx` passed in 14s", 5),
    ],
  }),
  entry("lb-lease", {
    section: "repo",
    repository: ARMADA,
    kind: "gotcha",
    text: "Lease a slot before the first write. Writes before it are refused.",
    confirmed_by: 13,
    first_seen: ago(24 * 20),
    last_confirmed: ago(3),
    last_tripped: ago(26),
    evidence: [
      row("c39d10", "Fork from a selection", "turn 6, Edit", "Refused: no slot leased for this Session", 26),
      row("c39d10", "Fork from a selection", "turn 8, lease_slot", "slot-4 leased, then the same Edit went through", 26),
      row("8a4e22", "Plan downshift at 80%", "turn 3, Write", "Refused: no slot leased for this Session", 24 * 6),
    ],
    history: [{ text: "A write needs a leased slot.", until: ago(26), by: "distil" }],
  }),
  entry("lb-graph", {
    section: "repo",
    repository: ARMADA,
    kind: "convention",
    text: "Ask the code graph what a change breaks before the edit and again before the commit.",
    confirmed_by: 4,
    last_confirmed: ago(4),
    evidence: [row("5d71be", "Rename Watched pulls to Followed", "turn 12, gitnexus impact", "Named 14 callers before the rename", 4)],
  }),
  entry("lb-check-test", {
    section: "repo",
    repository: ARMADA,
    kind: "gotcha",
    text: "`armada check test <name>` needs a Fleet built after #2200. An older one runs the whole suite.",
    state: "candidate",
    confirmed_by: 1,
    first_seen: ago(20),
    last_confirmed: ago(20),
    evidence: [row("0b8f3e", "Session usage drivers", "turn 22, Bash", "`armada check test usage_rules` ran 412 tests", 20)],
  }),
  entry("lb-merge-line", {
    section: "repo",
    repository: ARMADA,
    kind: "command",
    text: "Land with `scripts/land`. The merge line takes no new branches.",
    state: "faded",
    confirmed_by: 5,
    first_seen: ago(24 * 40),
    last_confirmed: ago(24 * 33),
    last_served: ago(24 * 31),
    evidence: [row("aa0d17", "Merge line freshness", "turn 30, Bash", "`scripts/land` queued the branch", 24 * 33)],
  }),
  entry("lb-ledger-migrate", {
    section: "repo",
    repository: LEDGER.root,
    kind: "command",
    text: "Run `make migrate` before the tests. A fresh checkout has no schema.",
    confirmed_by: 3,
    last_confirmed: ago(70),
    evidence: [
      row("3f2c88", "Monthly export in CSV", "turn 4, Bash", "`make test` failed: relation \"entries\" does not exist", 24 * 5),
      row("3f2c88", "Monthly export in CSV", "turn 5, Bash", "`make migrate && make test` passed", 24 * 5),
    ],
  }),
  entry("lb-ledger-env", {
    section: "repo",
    repository: LEDGER.root,
    kind: "environment",
    text: "Postgres 16 runs on port 5433 here, not 5432.",
    paths: [".env.example", "config/database.yml"],
    confirmed_by: 2,
    last_confirmed: ago(70),
    evidence: [row("3f2c88", "Monthly export in CSV", "turn 3, Bash", "`psql -p 5433` connected", 24 * 5)],
  }),
  entry("lb-fmt-habit", {
    section: "habits",
    kind: "habit",
    text: "Never run rustfmt or cargo fmt.",
    confirmed_by: 9,
    first_seen: ago(24 * 14),
    last_confirmed: ago(8),
    edited: true,
    evidence: [row("91be0c", "Check runs table", "turn 2, owner", "\"never run rustfmt, nor brief agents to\"", 24 * 14)],
    history: [{ text: "Leave Rust formatting alone unless asked.", until: ago(24 * 10), by: "person" }],
  }),
  entry("lb-walk", {
    section: "habits",
    kind: "habit",
    text: "Walk a visual change in the mock before it lands.",
    confirmed_by: 11,
    first_seen: ago(24 * 21),
    last_confirmed: ago(4),
    pinned: true,
    evidence: [
      row("44c7a2", "Dispatch panel", "turn 9, owner", "\"show me the walk first\"", 24 * 21),
      row("71f09a", "Walk window keeps two rows", "turn 25, show_window", "Walk approved, then landed", 4),
    ],
  }),
  entry("lb-copy", {
    section: "habits",
    kind: "convention",
    text: "UI copy states facts. No sentence explains what a thing is.",
    state: "faded",
    confirmed_by: 3,
    first_seen: ago(24 * 50),
    last_confirmed: ago(24 * 35),
    last_served: ago(24 * 32),
    evidence: [row("2e9d61", "Guides catalogue", "turn 14, owner", "\"I just want the app to show facts\"", 24 * 35)],
  }),
  entry("lb-playwright", {
    section: "machine",
    kind: "environment",
    text: "Playwright: Chrome is not in /Applications. Use the headless shell in ~/Library/Caches/ms-playwright.",
    confirmed_by: 8,
    first_seen: ago(24 * 12),
    last_confirmed: ago(6),
    last_tripped: ago(6),
    evidence: [
      row("b07e5d", "Pull request panel", "turn 31, Bash", "chromium.launch failed: Executable doesn't exist at /Applications/Google Chrome.app", 6),
      row("b07e5d", "Pull request panel", "turn 33, Bash", "launch with executablePath set to the headless shell passed", 6),
    ],
  }),
];

export const logbookPage: Scenario = {
  ...holding("logbook", "The Logbook: two repositories, Habits and This Mac, with a tripped entry, a pinned one, a candidate and two faded", [repository(), LEDGER]),
  logbook: ENTRIES,
};
