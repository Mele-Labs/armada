// What the mock serves the one side panel: a pull request as the forge reads it (whoever owns it), an issue, a bay and a Check.
// **These are fixtures.** A pull request the Reviews seed holds is read from it, so its page is the one the Reviews tab shows;
// the two the merge queue holds that no review is asked of get a small page of their own. Each file's patch is the seed's
// hunks, or a small invented one for those two, and a file with none says the forge's sentence. Built per window, so a
// comment written in one window is gone with it.

import type { MergeLines, PullRequestEntry, PullRequestFilePatch, PullRequestPage } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";
import type { SessionAttachment } from "@armada/screens/src/draft/sessions";

import { patchOfLines } from "../pr-review/patch";
import type { BayRead, CheckRead, IssueRead, PanelReads, PatchesRead, PullRequestRead } from "../shared-panel/reads";
import { reviewSeed } from "./pr-review-pages";

const minutesAgo = (minutes: number): string => new Date(Date.now() - minutes * 60_000).toISOString();
const ARMADA = repository().manifest!.id;
const ROOT = repository().root;
const FORGE = "https://forge.example/mele-labs/armada";
const BARE = "https://git.example/armada/pull/";

type Held = Extract<SessionAttachment, { kind: "pull_request" }>;

// ---- Pull requests the Reviews seed does not hold -----------------------------------------------------------

const row = (name: string, state: "passed" | "failed" | "running", seconds: number | undefined, number: number, at: number) => ({
  name,
  state,
  ...(seconds === undefined ? {} : { seconds }),
  address: `${FORGE}/pull/${number}/checks?check_run_id=${at + 1}`,
});

function pageOf(number: number, author: string, opened: number, body: string): PullRequestPage {
  return {
    manifest_id: ARMADA,
    number,
    base: "main",
    author,
    created_at: minutesAgo(opened),
    additions: 5,
    deletions: 1,
    mergeable: "mergeable",
    review: "none",
    approvals: 0,
    body,
    conversation: [{ kind: "comment", author: "nick", at: minutesAgo(opened - 12), body: "Read the migration against 0043 before it goes in." }],
    checks: [row("build", "passed", 131, number, 0), row("test · rust", "passed", 242, number, 1), row("test · bridge", "running", undefined, number, 2)],
    files: [
      { path: "crates/store/migrations/0044.sql", status: "added", additions: 3, deletions: 0 },
      { path: "crates/store/src/pause.rs", status: "modified", additions: 2, deletions: 1 },
      { path: "docs/pause-marker.png", status: "added", additions: 0, deletions: 0 },
    ],
    commits: [{ sha: "a41c9d2", headline: "Add the pause marker column", author, at: minutesAgo(opened - 4) }],
  };
}

/** What the forge says of a file it sent no patch for, in Fleet's words. */
const NO_PATCH = "The forge sent no patch for this file. It is binary, or the change is too large for the forge to show.";

/** The patches of a page `pageOf` drew, by path. A path absent here has none. */
const HUB_PATCHES: Record<string, string> = {
  "crates/store/migrations/0044.sql": "@@ -0,0 +1,3 @@\n+ALTER TABLE sessions\n+  ADD COLUMN paused_at TEXT;\n+CREATE INDEX sessions_paused ON sessions (paused_at);",
  "crates/store/src/pause.rs": "@@ -10,4 +10,5 @@\n pub struct Pause {\n-    at: Instant,\n+    at: SystemTime,\n+    marker: Option<String>,\n }",
};

const HUB: Record<number, { title: string; branch: string; author: string; opened: number; body: string; queued: boolean; checks: Held["checks"]; merged?: true }> = {
  1890: { title: "Gate policy on every run", branch: "fleet/gate-policy-every-run", author: "claude", opened: 70, body: "Runs the gate on every run, not only the last.", queued: true, checks: { state: "passed" } },
  1892: { title: "Add the pause marker column", branch: "fleet/pause-marker-column", author: "claude", opened: 30, body: "Stores a pause marker so a quiet Job survives a reload.\n\nCloses #2214.", queued: false, checks: { state: "pending" } },
  // The `progress-issues` Sessions' pull requests.
  2248: { title: "Read the pause marker on load", branch: "fleet/read-pause-marker", author: "claude", opened: 140, body: "Reads the pause marker when the list loads.\n\nCloses #2214.", queued: false, checks: { state: "passed" }, merged: true },
  2251: { title: "Window the Cockpit job list", branch: "bridge/window-job-list", author: "claude", opened: 50, body: "Draws only the rows in view.", queued: false, checks: { state: "passed" } },
  2253: { title: "Drop the list's second render", branch: "bridge/one-list-render", author: "claude", opened: 12, body: "The list rendered twice on each tick.", queued: false, checks: { state: "pending" } },
  2256: { title: "Clear Helm's cache on a repository switch", branch: "helm/clear-on-switch", author: "claude", opened: 25, body: "Closes #2212.", queued: false, checks: { state: "failed", failing: "test · bridge" } },
  2260: { title: "Name Helm's repository in its header", branch: "helm/header-repository", author: "claude", opened: 8, body: "Shows which repository Helm reads.", queued: false, checks: { state: "passed" } },
  2281: { title: "Keep Helm's draft across a resize", branch: "helm/keep-draft-on-resize", author: "claude", opened: 6, body: "Closes #2271.", queued: false, checks: { state: "pending" } },
  2282: { title: "Clear the old answer when Helm asks again", branch: "helm/clear-old-answer", author: "claude", opened: 90, body: "Closes #2272.", queued: false, checks: { state: "passed" }, merged: true },
  // The `progress-issues-jira` Session's pull requests, which name their tickets by key.
  2301: { title: "Round the checkout total once", branch: "checkout/round-once", author: "claude", opened: 40, body: "Works ABC-41.", queued: false, checks: { state: "pending" } },
  2302: { title: "Keep the coupon code through a resize", branch: "checkout/keep-coupon", author: "claude", opened: 180, body: "Works ABC-42.", queued: false, checks: { state: "passed" }, merged: true },
  2303: { title: "Show the rounded total in the receipt", branch: "checkout/receipt-total", author: "claude", opened: 15, body: "Works ABC-41.", queued: false, checks: { state: "passed" } },
};

// ---- Issues ----------------------------------------------------------------------------------------------------

const ISSUES: Record<string, IssueRead> = {
  2214: {
    number: "2214",
    title: "Sessions list drops a quiet Session on reload",
    repository: "mele-labs/armada",
    address: "https://forge.invalid/issues/2214",
    state: "open",
    author: "nick",
    openedAt: minutesAgo(26 * 60),
    labels: ["bug", "bridge"],
    assignees: [],
    body: "A Session that has been quiet for an hour is gone from the list after the window reloads. Its page still opens by address.\n\nSeen with two Sessions paused overnight.",
    comments: [
      { author: "marta", at: minutesAgo(20 * 60), body: "Reproduced. The list is rebuilt from Sessions that wrote in the last hour." },
      { author: "nick", at: minutesAgo(18 * 60), body: "A pause marker would let the list keep it." },
    ],
    linked: [{ number: 1892, title: "Add the pause marker column", address: `${BARE}1892`, state: "open" }],
  },
  2212: {
    number: "2212",
    title: "Helm answers about the wrong repository after a switch",
    repository: "mele-labs/armada",
    address: "https://forge.invalid/issues/2212",
    state: "open",
    author: "marta",
    openedAt: minutesAgo(3 * 24 * 60),
    labels: ["bug"],
    assignees: ["nick"],
    body: "After pointing Helm at another repository, its next answer still reads the first one's issues.",
    comments: [],
    linked: [{ number: 2256, title: "Clear Helm's cache on a repository switch", address: `${BARE}2256`, state: "open" }],
  },
  2259: {
    number: "2259",
    title: "Helm composer keeps the old repository chip",
    repository: "mele-labs/armada",
    address: "https://forge.invalid/issues/2259",
    state: "closed",
    author: "nick",
    openedAt: minutesAgo(2 * 24 * 60),
    labels: ["bridge"],
    assignees: [],
    body: "After a switch the composer still shows the first repository's chip.",
    comments: [{ author: "nick", at: minutesAgo(6 * 60), body: "Gone since the chip reads the switch." }],
    linked: [],
  },
  2271: {
    number: "2271",
    title: "Helm drops its draft when the window resizes",
    repository: "mele-labs/armada",
    address: "https://forge.invalid/issues/2271",
    state: "open",
    author: "nick",
    openedAt: minutesAgo(5 * 60),
    labels: ["bug", "bridge"],
    assignees: [],
    body: "A half-written question in Helm is gone after the window is resized.",
    comments: [],
    linked: [{ number: 2281, title: "Keep Helm's draft across a resize", address: `${BARE}2281`, state: "open" }],
  },
  2272: {
    number: "2272",
    title: "Helm shows the previous answer while a new one loads",
    repository: "mele-labs/armada",
    address: "https://forge.invalid/issues/2272",
    state: "closed",
    author: "marta",
    openedAt: minutesAgo(4 * 24 * 60),
    labels: ["bridge"],
    assignees: [],
    body: "Asking again leaves the last answer on screen until the new one arrives.",
    comments: [],
    linked: [{ number: 2282, title: "Clear the old answer when Helm asks again", address: `${BARE}2282`, state: "merged" }],
  },
  2273: {
    number: "2273",
    title: "Helm cannot be reached by the keyboard from the Cockpit",
    repository: "mele-labs/armada",
    address: "https://forge.invalid/issues/2273",
    state: "open",
    author: "nick",
    openedAt: minutesAgo(2 * 60),
    labels: ["bridge"],
    assignees: [],
    body: "Opening Helm takes the pointer.",
    comments: [],
    linked: [],
  },
  // Tickets on a tracker that names them by key, the `progress-issues-jira` Session's.
  "ABC-41": {
    number: "ABC-41",
    title: "Checkout total rounds twice",
    repository: "mele-labs/armada",
    address: "https://tracker.invalid/browse/ABC-41",
    state: "open",
    author: "marta",
    openedAt: minutesAgo(26 * 60),
    labels: ["bug"],
    assignees: [],
    body: "A basket of three items shows a total one cent above the sum of its lines.",
    comments: [],
    linked: [
      { number: 2301, title: "Round the checkout total once", address: `${BARE}2301`, state: "open" },
      { number: 2303, title: "Show the rounded total in the receipt", address: `${BARE}2303`, state: "open" },
    ],
  },
  "ABC-42": {
    number: "ABC-42",
    title: "Coupon field drops its code on a resize",
    repository: "mele-labs/armada",
    address: "https://tracker.invalid/browse/ABC-42",
    state: "closed",
    author: "nick",
    openedAt: minutesAgo(3 * 24 * 60),
    labels: ["bug"],
    assignees: [],
    body: "A coupon code typed into checkout is gone after the window is resized.",
    comments: [],
    linked: [{ number: 2302, title: "Keep the coupon code through a resize", address: `${BARE}2302`, state: "merged" }],
  },
};

// ---- Bays --------------------------------------------------------------------------------------------------------

const base = { base: "main", warm: true, path: "", uncommitted: [] as string[], unpushed: [] as string[] };
const at = (manifest: string, slot: number) => `/Users/user/${manifest}/.armada/slots/slot-${slot}`;

const BAYS: Record<string, BayRead> = {
  [`${ARMADA}/1`]: { ...base, name: "slot-1", repository: "armada", state: "leased", word: "Held", holder: { id: "p1", title: "Gate policy on every run" }, branch: "fleet/gate-policy-every-run", heldFor: "1h 20m", behind: 0, path: at("armada", 1), history: [{ who: "Gate policy on every run", span: "1h 20m", branch: "fleet/gate-policy-every-run" }, { who: "Release notes", span: "25m", branch: "docs/release-notes" }] },
  [`${ARMADA}/2`]: { ...base, name: "slot-2", repository: "armada", state: "leased", word: "Held", holder: { id: "q1", title: "Cache key" }, branch: "fix/cache-key", heldFor: "17m", behind: 0, path: at("armada", 2), history: [{ who: "Cache key", span: "17m", branch: "fix/cache-key" }] },
  [`${ARMADA}/3`]: { ...base, name: "slot-3", repository: "armada", state: "leased", word: "Held", holder: { id: "f1", title: "Fix the reconnect wait" }, branch: "fleet/reconnect-wait", heldFor: "33m", behind: 0, path: at("armada", 3), history: [{ who: "Fix the reconnect wait", span: "33m", branch: "fleet/reconnect-wait" }] },
  [`${ARMADA}/4`]: { ...base, name: "slot-4", repository: "armada", state: "leased", word: "Held", holder: { id: "r1", title: "Pin the store clock" }, branch: "fix/pin-store-clock", heldFor: "16m", behind: 0, path: at("armada", 4), history: [{ who: "Pin the store clock", span: "16m", branch: "fix/pin-store-clock" }] },
  [`${ARMADA}/5`]: { ...base, name: "slot-5", repository: "armada", state: "free", word: "Free", warm: false, behind: 2, path: at("armada", 5), history: [] },
  [`${ARMADA}/6`]: {
    ...base,
    name: "slot-6",
    repository: "armada",
    state: "stranded",
    word: "Stranded",
    branch: "session/frame-the-reply",
    heldFor: "3h",
    behind: 4,
    path: at("armada", 6),
    uncommitted: ["crates/ipc/src/frame.rs", "notes/frame.md"],
    unpushed: ["e08c4d1 Frame the reply once"],
    history: [{ who: "Frame the reply", span: "3h", branch: "session/frame-the-reply" }],
  },
};

// ---- Checks ------------------------------------------------------------------------------------------------------

const LOG_RED = [
  "$ pnpm exec vitest run --project desktop",
  " RUN  v3.2.4 /Users/user/armada/apps/desktop",
  "",
  " FAIL  resize.test.ts > keeps the sidebar width",
  "AssertionError: expected 312 to be 288",
  "  at resize.test.ts:41:19",
  "",
  " Test Files  1 failed | 211 passed",
  "      Tests  1 failed | 1904 passed",
];
const LOG_OK = ["$ cargo test --workspace", "   Compiling fleet v0.1.0", "    Finished test profile in 41.2s", "     Running unittests", "test result: ok. 1312 passed; 0 failed"];

function checkRead(name: string): CheckRead {
  const red = name === "desktop_test";
  const pull = (number: number) => ({ number, address: `${FORGE}/pull/${number}` });
  return {
    name,
    repository: "mele-labs/armada",
    state: red ? "failed" : "passed",
    runs: red
      ? [
          { on: "main", state: "failed", seconds: 412, ago: "1m" },
          { on: "fleet/reconnect-wait", state: "failed", seconds: 398, ago: "22m", pull: pull(1896) },
          { on: "fleet/gate-policy-every-run", state: "passed", seconds: 405, ago: "2h", pull: pull(1890) },
        ]
      : [
          { on: "main", state: "passed", seconds: 220, ago: "1h" },
          { on: "marta/plan-window-cache", state: "passed", seconds: 231, ago: "2h", pull: pull(1899) },
          { on: "fleet/reconnect-wait", state: "passed", seconds: 226, ago: "3h", pull: pull(1896) },
        ],
    log: red ? { run: "main", lines: LOG_RED } : { run: "main", lines: LOG_OK },
  };
}

// ---- The reads ---------------------------------------------------------------------------------------------------

export function sharedPanelReads(lines: MergeLines): PanelReads {
  const seeded = reviewSeed().prs;
  const pages = new Map<number, PullRequestPage>();
  let asked = 0;
  const queued = new Set((lines.lines.flatMap((one) => one.hub?.pull_requests ?? []).filter((one) => one.queue !== undefined)).map((one) => one.number));

  const read = (number: number): PullRequestRead | undefined => {
    const mock = seeded.find((one) => one.number === number);
    if (mock !== undefined) {
      const own = HUB[number]?.body;
      const page = pages.get(number) ?? (own === undefined ? mock.page : { ...mock.page, body: own });
      const failing = mock.page.checks.filter((one) => one.state === "failed").map((one) => one.name);
      const facts: Held = {
        kind: "pull_request",
        number,
        title: mock.title,
        branch: mock.branch,
        address: mock.address,
        checks: mock.checks === "failed" ? { state: "failed", failing: failing.join(", ") } : mock.checks === "running" ? { state: "pending" } : { state: "passed" },
        state: mock.draft ? "draft" : "open",
        auto: false,
        ...(queued.has(number) ? { queued: true } : {}),
        body: own ?? mock.body,
      };
      return { repository: mock.repository, ...(mock.repository.endsWith("/armada") ? { root: ROOT } : {}), facts, page };
    }
    const hub = HUB[number];
    if (hub === undefined) return undefined;
    const page = pages.get(number) ?? pageOf(number, hub.author, hub.opened, hub.body);
    const facts: Held = { kind: "pull_request", number, title: hub.title, branch: hub.branch, address: `${BARE}${number}`, checks: hub.checks, state: hub.merged === true ? "merged" : "open", auto: false, ...(hub.queued ? { queued: true } : {}), body: hub.body };
    return { repository: "mele-labs/armada", root: ROOT, facts, page };
  };

  const patches = (number: number): PatchesRead => {
    const was = read(number);
    if (was === undefined) return { ok: false, said: "The forge could not find this pull request." };
    const mock = seeded.find((one) => one.number === number);
    const hunks = mock?.explanation.themes.flatMap((theme) => theme.changes.flatMap((change) => change.hunks)) ?? [];
    const patchOf = (path: string): string | undefined => {
      const own = hunks.filter((hunk) => hunk.path === path);
      return mock === undefined ? HUB_PATCHES[path] : own.length === 0 ? undefined : own.map((hunk) => patchOfLines(hunk.lines)).join("\n");
    };
    const files = was.page.files.map((file): PullRequestFilePatch => {
      const patch = patchOf(file.path);
      return { manifest_id: was.page.manifest_id, number, path: file.path, ...(patch === undefined ? { said: NO_PATCH } : { patch }) };
    });
    return { ok: true, files };
  };

  return {
    lines,
    numbered: [
      { kind: "issue", number: "2214", title: "Sessions list drops a quiet Session on reload", url: "https://forge.invalid/issues/2214" },
      { kind: "issue", number: "2212", title: "Helm answers about the wrong repository after a switch", url: "https://forge.invalid/issues/2212" },
      { kind: "pull_request", number: "1892", title: "Add the pause marker column", url: `${BARE}1892` },
    ],
    // The forge is slow the second time it is asked about #1892, and answers with a comment the first read did not hold: what a panel opened again draws from its last read, and what the refresh then adds.
    pullRequest: async (item) => {
      const was = read(item.number);
      if (item.number !== 1892 || was === undefined) return was;
      asked += 1;
      if (asked === 1) return was;
      await new Promise((resolve) => setTimeout(resolve, 2500));
      const entry: PullRequestEntry = { kind: "comment", author: "marta", at: new Date().toISOString(), body: "Refreshed from the forge." };
      return { ...was, page: { ...was.page, conversation: [...was.page.conversation, entry] } };
    },
    pullRequestPatches: async (item) => patches(item.number),
    writePullRequest: async (item, write) => {
      const was = read(item.number);
      if (was === undefined) return { ok: false, said: "The forge could not find this pull request." };
      const entry: PullRequestEntry =
        write.kind === "comment"
          ? { kind: "comment", author: "nick", at: new Date().toISOString(), body: write.body ?? "" }
          : { kind: "review", author: "nick", at: new Date().toISOString(), verdict: write.kind === "approve" ? "approved" : "changes_requested", body: write.body ?? "" };
      const page: PullRequestPage = { ...was.page, conversation: [...was.page.conversation, entry], ...(write.kind === "approve" ? { review: "approved" as const, approvals: was.page.approvals + 1 } : {}) };
      pages.set(item.number, page);
      return { ok: true, page };
    },
    actOnPullRequest: async () => true,
    issue: async (item) => ISSUES[item.number],
    bay: async (item) => BAYS[`${item.manifestId}/${item.slot}`],
    check: async (item) => {
      const name = item.name ?? item.id.split(":").at(-1) ?? item.id;
      return checkRead(name);
    },
  };
}
