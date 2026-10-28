// The Logbook as it reads after a few weeks: long entries in use, one confirmed by many Jobs with more evidence than
// fits, and the retro items on their tab. `docs/concepts/logbook.md`. The walk `logbook-rework` plays over it.

import type { LogbookEntry, LogbookEvidence, LogbookKind, LogbookSection, RepositorySummary } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";
import { retroFixtures } from "@armada/screens/src/fixtures/retros";

import { holding } from "../holding";
import type { Scenario } from "../moment";

const ARMADA = repository().root;
const LEDGER: RepositorySummary = { root: "/Users/user/ledger", records_root: "/Users/user/Library/Application Support/Armada/records/ledger" };
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
  entry(
    "lb-preview-restart",
    "A restart the owner must run goes in `waiting_for` as a run item with `scripts/preview --restart`, so Bridge shows Refresh preview on his card and the Sessions page. Never offer a plain restart onto main: it drops every in-flight branch from the preview and strands any walk he has open. When the restart is part of a longer chain, give it its own item rather than folding it into a compound command line, because a compound line loses the Refresh preview key. After the restart, check that `armada --version` names the preview commit before telling him it is done, since a stale binary on the PATH answers the old version. If a Drone is working when the restart runs, pass `--adopt` so it carries on instead of being stopped. A walk served from a slot survives the restart; a walk served from the preview checkout does not, so show it again. Do not chain the restart with a check: the check reads the old Fleet.",
    { title: "Restart through the preview, never onto main", kind: "convention", confirmed_by: 9, last_confirmed: ago(0.5), last_tripped: ago(30), evidence: MANY.slice(0, 9) },
  ),
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


/** The many entries a Logbook gathers over weeks: each a title and its rule, so the log runs well past a screen. */
const MORE: [LogbookSection, LogbookKind, string, string][] = [
  ["repo", "command", "Build the desktop app with pnpm build", "Run `pnpm build` from `apps/desktop`; the root build skips the renderer."],
  ["repo", "gotcha", "The merge queue reruns ci", "A green pull request still waits for the queue's own ci run before it lands."],
  ["repo", "convention", "Cite beads issues by id", "Cite an issue by its beads id such as `armada-a3f2`; it has no web page."],
  ["repo", "command", "Regenerate the protocol mirror", "After a Rust wire change run `cargo xtask protocol --write` and commit the TypeScript it writes."],
  ["repo", "gotcha", "Storybook needs its own port", "Storybook fails on 6006 when the mock is up; pass `--port 6106`."],
  ["repo", "convention", "Tokens only in component CSS", "Every value in a component stylesheet is a token; the verify gate refuses a raw pixel."],
  ["repo", "environment", "Node 22 from .nvmrc", "The repo pins Node 22; an older Node fails `pnpm install` on the engines field."],
  ["repo", "command", "Play a walk before showing it", "Run the walk's test with `-t <walk>` across the four walks files before calling show_window."],
  ["repo", "gotcha", "Vite holds the old CSS after a rename", "A renamed stylesheet keeps the old one in the page until a full reload."],
  ["repo", "convention", "One question at a time", "Put each decision to the owner on its own, with options, at the end of the turn."],
  ["repo", "command", "Index the code graph after a merge", "Run `gitnexus analyze --index-only` after merging main, or impact answers are stale."],
  ["repo", "gotcha", "Hooks refuse git branch -D", "Deleting a branch is refused; finish_up gives back merged branches."],
  ["repo", "convention", "Docs ship with the change", "A doc the change made wrong is fixed in the same pull request."],
  ["repo", "environment", "Rust nightly for xtask", "xtask builds on the pinned nightly in `rust-toolchain.toml`; stable fails on a let chain."],
  ["repo", "command", "Verify foundations against main", "Read `cargo xtask verify-foundations` as a delta against main, not as a colour."],
  ["repo", "gotcha", "Fixtures need a second repository", "A scenario with one repository hides the section rows; give it two."],
  ["repo", "convention", "No aggregate counts", "Never draw a count beside the items it counts."],
  ["repo", "command", "Type-check screens separately", "After editing `packages/screens` run `pnpm exec tsc --noEmit` there too."],
  ["repo", "gotcha", "jsdom has no layout", "A walk test passes on a page that is visibly broken; look in a browser too."],
  ["repo", "convention", "Icons from the registry", "A state is a registry glyph with a tooltip, never an emoji."],
  ["repo", "command", "Land with auto-merge", "Open the pull request and run `gh pr merge <n> --merge --auto`."],
  ["repo", "gotcha", "The slot guard reads the lease", "A write outside the leased slot is refused even when the path is under the repo."],
  ["repo", "convention", "Facts, not narration, in UI copy", "UI strings state facts; no sentence explains what a thing is."],
  ["repo", "environment", "Electron 38 needs macOS 13", "The desktop app does not launch on macOS 12; the mock in a browser does."],
  ["repo", "command", "Run one Rust test by name", "`armada check test <test_name>` runs one test in the background."],
  ["repo", "gotcha", "Preview restarts drop open walks", "A preview restart closes the walk window; show it again after."],
  ["repo", "convention", "Keyboard first", "Every new surface is fully keyboard driven, with its keys named in the walk."],
  ["repo", "command", "Serve the mock on a free port", "`pnpm exec vite --config vite.mock.config.ts --port <free> --strictPort` from `apps/desktop`."],
  ["repo", "gotcha", "Sticky table cells need a solid fill", "A sticky header with a clear background shows the rows through it."],
  ["repo", "convention", "Walk before landing", "A change the owner would see waits for his walk before it lands."],
  ["habits", "habit", "Short walks for fixes", "A fix gets a few steps straight to it, never the whole feature walk."],
  ["habits", "habit", "No windows on his screen", "Never open a visible Electron window from a test or harness."],
  ["habits", "habit", "Ask once", "A question goes in the prompt or in Needs you, never both."],
  ["habits", "habit", "Two agents at most", "Run at most two child agents at once."],
  ["habits", "habit", "No gradients or glows", "Styling stays flat: no gradients, glows, sheen or grain."],
  ["habits", "habit", "Build, don't file", "An idea he floats goes to an agent to build, not to an issue."],
  ["habits", "habit", "Root cause first", "A slowness question gets its one structural cause before any patch."],
  ["habits", "habit", "Restart with the preview", "A restart he must run is `scripts/preview --restart`, never a plain restart."],
  ["machine", "environment", "No timeout command", "`timeout` is not installed; run the command without it."],
  ["machine", "environment", "Quote globs in zsh", "Quote every glob zsh could expand, in paths as well as `--include=`."],
  ["machine", "environment", "SSH fails while locked", "Git over SSH fails with the screen locked; the repo uses HTTPS through gh."],
  ["machine", "environment", "Playwright headless shell", "Chrome is not in /Applications; use the headless shell in the Playwright cache."],
];

const LEDGER_MORE: [LogbookKind, string, string][] = [
  ["command", "Run make migrate before tests", "Run `make migrate` before the tests; a fresh checkout has no schema."],
  ["environment", "Postgres on 5433", "Postgres 16 runs on port 5433 here, not 5432."],
  ["gotcha", "CSV export needs UTF-8 BOM", "Excel misreads the export without a BOM; `export.rs` writes one."],
  ["convention", "Money in cents", "Every amount is an integer of cents; never a float."],
  ["command", "Seed with make seed", "`make seed` loads a month of entries for local work."],
  ["gotcha", "Timezones in the ledger", "Entries are stored in UTC and shown in the account's zone."],
  ["convention", "One migration a change", "A change adds at most one migration file."],
  ["command", "Lint with make lint", "`make lint` runs clippy and sqlfluff together."],
  ["gotcha", "Reports cache for an hour", "A report reads stale for an hour after an import; `make bust` clears it."],
  ["environment", "Redis for the queue", "The import queue needs Redis on 6380."],
  ["convention", "Accounts are soft deleted", "Never delete an account row; set `closed_at`."],
  ["command", "Snapshot tests with insta", "Accept changed snapshots with `cargo insta review`."],
];

function many(): LogbookEntry[] {
  const at = (n: number) => 1 + n * 3.5;
  const own = MORE.map(([section, kind, title, text], n) =>
    entry(`lb-more-${n}`, text, {
      title,
      section,
      kind,
      ...(section === "repo" ? {} : { repository: undefined }),
      confirmed_by: 2 + ((n * 7) % 11),
      last_confirmed: ago(at(n)),
      ...(n % 9 === 4 ? { last_tripped: ago(at(n) + 2) } : {}),
      ...(n % 13 === 6 ? { locked: true } : {}),
    }),
  );
  const ledger = LEDGER_MORE.map(([kind, title, text], n) =>
    entry(`lb-ledger-${n}`, text, { title, kind, repository: LEDGER.root, confirmed_by: 2 + (n % 5), last_confirmed: ago(at(n) + 10) }),
  );
  return [...own, ...ledger];
}

export const logbookLong: Scenario = {
  ...holding("logbook-long", "The Logbook after a few weeks: some 75 entries over two repositories, one with evidence from many Jobs, and the retro items on their tab", [repository(), LEDGER]),
  logbook: [...ENTRIES, ...many()],
  retros,
  lessons,
};
