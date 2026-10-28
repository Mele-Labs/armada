// The Logbook as it reads after a few weeks: long entries in use, one confirmed by many Jobs with more evidence than
// fits, and the retro items on their tab. `docs/concepts/logbook.md`. The walk `logbook-rework` plays over it.

import type { LogbookEntry, LogbookEvidence } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";
import { retroFixtures } from "@armada/screens/src/fixtures/retros";

import { holding } from "../holding";
import type { Scenario } from "../moment";

const ARMADA = repository().root;
const { retros, lessons } = retroFixtures();

const HOUR = 3_600_000;
const ago = (hours: number) => new Date(Date.now() - hours * HOUR).toISOString();

const row = (job_id: string, job_title: string, cite: string, says: string, hours: number): LogbookEvidence => ({ job_id, job_title, cite, says, at: ago(hours) });

function entry(id: string, text: string, fields: Partial<LogbookEntry> = {}): LogbookEntry {
  return {
    id,
    section: "repo",
    repository: ARMADA,
    kind: "gotcha",
    text,
    evidence: [row("c39d10", "Fork from a selection", "ran:6", "Refused by the slot guard", 3)],
    confirmed_by: 3,
    first_seen: ago(24 * 4),
    last_confirmed: ago(3),
    last_served: ago(1),
    state: "standing",
    locked: false,
    edited: false,
    history: [],
    ...fields,
  };
}

/** Many Jobs, several rows from some: more than the right side holds at once. */
const MANY: LogbookEvidence[] = [
  row("pr-reviews", "When I open a PR in the reviews tab, it shouldn't open in the browser", "ran:26", "`cp f f.new` refused", 24 * 4),
  row("pr-reviews", "When I open a PR in the reviews tab, it shouldn't open in the browser", "ran:27", "`git show HEAD:path > f` refused", 24 * 4),
  row("pr-reviews", "When I open a PR in the reviews tab, it shouldn't open in the browser", "refusal:1", "Overwrites outside /tmp", 24 * 4),
  row("pr-reviews", "When I open a PR in the reviews tab, it shouldn't open in the browser", "refusal:2", "Overwrites outside /tmp", 24 * 4),
  row("pr-reviews", "When I open a PR in the reviews tab, it shouldn't open in the browser", "refusal:3", "Overwrites outside /tmp", 24 * 4),
  row("long-questions", "Sometimes when questions are long the answer buttons are out of view", "ran:55", "`cp Ask.tsx Ask.tsx.new` refused", 24 * 4),
  row("long-questions", "Sometimes when questions are long the answer buttons are out of view", "ran:57", "Copied to /tmp, then it went through", 24 * 4),
  row("flight-overflow", "Flight Control Overflow", "ran:42", "Refused in the slot", 24 * 4),
  row("flight-overflow", "Flight Control Overflow", "ran:46", "Copied to /tmp, then it went through", 24 * 4),
  row("annotate", "Annotate sketches, documents and images for jobs", "ran:20", "Refused in the slot", 24 * 4),
  row("logbook", "Armada Logbook", "subagent:11", "The subagent hit the same refusal", 24 * 4),
  row("teaching", "Teaching Armada", "refusal:2", "Overwrites outside /tmp", 24 * 4),
  row("teaching", "Teaching Armada", "refusal:3", "Overwrites outside /tmp", 24 * 4),
  row("layout-settings", "Some of these layout settings don't apply anymore", "ran:44", "Refused in the slot", 24 * 3),
  row("sessions-absorb", "Sessions absorb Jobs", "ran:12", "Refused in the slot", 24 * 2),
  row("cockpit-trust", "Cockpit trust", "ran:31", "Copied to /tmp, then it went through", 24),
  row("quick-dispatch", "Quick dispatch", "ran:8", "Refused in the slot", 12),
  row("palette", "Command palette like Raycast", "ran:19", "Copied to /tmp, then it went through", 1),
];

const ENTRIES: LogbookEntry[] = [
  entry(
    "lb-head-backup",
    "Putting HEAD's version of a file in place with a backup beside it in the slot (`cp $F $F.new`, then `git show HEAD:path > $F`) is refused. Copy the file to /tmp first, then `git show HEAD:path > f`, run the test, and copy it back.",
    { title: "Copy to /tmp before restoring from HEAD", kind: "command", confirmed_by: 15, last_confirmed: ago(0.35), last_served: ago(0.03), evidence: MANY },
  ),
  entry("lb-start-server", "Fleet `start_server` rejected `job` given as `job`, as the slot path, and as the session uuid, all with `fleet.not_a_checkout`. Starting `pnpm exec vite --config vite.mock.config.ts --port <free port> --strictPort` directly from `apps/desktop` worked.", { title: "Start the mock with vite, not start_server", last_confirmed: ago(0.35), last_tripped: ago(5) }),
  entry("lb-merge", "Always pass `--merge`: `gh pr merge <n> --merge --auto`. Bare `gh pr merge <n> --auto` is refused by a hook.", { title: "Pass --merge to gh pr merge", kind: "convention", last_confirmed: ago(0.35) }),
  entry("lb-first-write", "The first write in a job is refused until Fleet moves the job into its leased slot (`.armada/slots/slot-N`). After that first refusal, stop for the turn: do not rerun the same command or issue another write. Once told you are in the slot, repeat it with `cd <slot>` first.", { title: "First write waits for the slot", last_confirmed: ago(0.35) }),
  entry("lb-playwright", "Playwright `browser_take_screenshot` refuses a path outside the repo root and its `.playwright-mcp`; `/tmp/fc-wider.png` was refused. Save screenshots under the slot or `.playwright-mcp/`.", { title: "Save screenshots inside the repo", last_confirmed: ago(0.35) }),
  entry("lb-tsc", "Type-check the desktop app before committing with both `pnpm exec tsc --noEmit -p tsconfig.web.json` and `-p tsconfig.node.json` from `apps/desktop`. After editing `packages/screens`, also run `pnpm exec tsc --noEmit` there.", { title: "Type-check web and node before committing", kind: "command", last_confirmed: ago(0.35) }),
  entry("lb-cwd", "The shell cwd can be a slot the job does not hold, and a relative `cd` can fail. Start every Bash with `cd` to the absolute path of the held slot; bare calls were refused by a hook.", { title: "Start every Bash with cd to the slot", last_confirmed: ago(3) }),
  entry("lb-rust-tests", "Run Rust tests only through `armada check test`: `armada check test <test_name>` for one test, `git diff --name-only origin/main...HEAD | armada check test --changed` for changed files. Both run in the background. Bare `cargo test` and `cargo nextest run` are stopped by a hook.", { title: "Rust tests only through armada check test", kind: "command", last_confirmed: ago(3), locked: true }),
  entry("lb-mod-version", "When a change touches `plugins/armada` (including `output-styles/armada.md`), change `MOD_VERSION` in `plugins/armada/hooks/facts.ts` and `version` in `plugins/armada/.claude-plugin/plugin.json` in the same commit. Bumping only plugin.json failed `the_version_the_mod_reports_is_the_plugins_own` in the merge queue.", { title: "Bump the mod version in both files", kind: "convention", last_confirmed: ago(3) }),
  entry("lb-sketch-icons", "`show_sketch` and the `sketch` in `record_plan` accept only icons from the allowed list (`rotate-cw`, `circle-dot` and `git-compare` were refused). Pick one the refusal names, such as `circle-check`, `flag`, `history` or `git-branch`.", { title: "Sketch icons come from the allowed list", last_confirmed: ago(3) }),
  entry("lb-vitest-dir", "Run vitest from the package directory with `pnpm exec vitest run <file>`. The same run from the slot root failed with `Command \"vitest\" not found`.", { title: "Run vitest from the package folder", kind: "command", last_confirmed: ago(5) }),
  entry("lb-verify-docs", "Run `cargo xtask verify-docs --write` and `cargo xtask verify-tokens --write` before pushing any `docs/` edit, then commit what they write.", { title: "Regenerate docs and tokens before pushing", kind: "command", last_confirmed: ago(6) }),
  entry("lb-worktree-remove", "Removing a git worktree is refused even when it sits under /tmp. Do not retry it or chain it with other commands; say in your reply which worktree needs removing and why.", { title: "Worktree removal is refused", last_confirmed: ago(8) }),
  entry("lb-kill", "`kill` and `pkill` are refused every time, even for a vite you started yourself. Say in your reply which process needs stopping.", { title: "kill and pkill are refused", last_confirmed: ago(9) }),
  entry("lb-fmt", "Never run rustfmt or cargo fmt.", { title: "No rustfmt", section: "habits", kind: "habit", last_confirmed: ago(8) }),
  entry("lb-walk", "Walk a visual change in the mock before it lands.", { title: "Walk visual changes first", section: "habits", kind: "habit", last_confirmed: ago(4), locked: true }),
  entry("lb-sed", "Plain `sed -i 's/…/…/' file` fails here with usage errors. For a short edit use the BSD form `sed -i '' 's|…|…|' file`.", { title: "Use the BSD form of sed -i", section: "machine", kind: "environment", last_confirmed: ago(6) }),
];

export const logbookLong: Scenario = {
  ...holding("logbook-long", "The Logbook after a few weeks: long entries, one with evidence from many Jobs, and the retro items on their tab", [repository()]),
  logbook: ENTRIES,
  retros,
  lessons,
};
