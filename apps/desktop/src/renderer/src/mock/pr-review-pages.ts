// What a mock forge holds of five pull requests for the review surface: three that teammates asked his review
// of, and two that agents of his opened. Each has a real-looking patch of several files, and one of the three
// has threads already, so a reply and a resolve have something to act on. **The patches are written by hand**
// and the files' figures are counted from them, so the tree and the diff cannot disagree.
//
// Built on the helpers `session-pr-pages.ts` has for the panel's tabs, which read the same shapes.

import type { PullRequestCheckRow, PullRequestCommit, PullRequestEntry, PullRequestPage } from "@armada/protocol";
import { drawnOf } from "@armada/screens/src/drawn-patch";

import type { FindingSeed, ReviewFile, ReviewPr, ReviewSeed, Thread } from "../pr-review/model";

const FORGE = "https://github.com/mele-labs";
const minutesAgo = (minutes: number): string => new Date(Date.now() - minutes * 60_000).toISOString();

const checks = (repository: string, number: number, rows: readonly (readonly [string, PullRequestCheckRow["state"], number?])[]): PullRequestCheckRow[] =>
  rows.map(([name, state, seconds], at) => ({
    name,
    state,
    ...(seconds === undefined ? {} : { seconds }),
    ...(state === "skipped" ? {} : { address: `${FORGE}/${repository}/pull/${number}/checks?check_run_id=${at + 1}` }),
  }));

const commits = (rows: readonly (readonly [string, string, string, number])[]): PullRequestCommit[] =>
  rows.map(([sha, headline, author, ago]) => ({ sha, headline, author, at: minutesAgo(ago) }));

type FileIn = readonly [path: string, status: ReviewFile["status"], patch: readonly string[]];

/** A file as the reader draws it: the patch split into the lines git wrote, and the figures counted from them. */
function fileOf([path, status, patch]: FileIn): ReviewFile {
  const drawn = drawnOf(`diff --git a/${path} b/${path}\n${patch.join("\n")}`, () => "");
  const lines = drawn.files[0]?.lines ?? [];
  return {
    path,
    status,
    lines,
    additions: lines.filter((one) => one.kind === "added").length,
    deletions: lines.filter((one) => one.kind === "removed").length,
  };
}

/** The place of the first line of `path` that holds `match`. A fixture names a line by what it says, not by a count. */
function lineOf(files: readonly ReviewFile[], path: string, match: string): number {
  const file = files.find((one) => one.path === path);
  const at = file?.lines.findIndex((one) => one.text.includes(match)) ?? -1;
  if (at < 0) throw new Error(`${path} has no line holding “${match}”`);
  return at;
}

type Spec = {
  repository: string;
  manifestId: string;
  number: number;
  title: string;
  branch: string;
  author: string;
  tab: ReviewPr["tab"];
  opened: number;
  body: string;
  checks: ReviewPr["checks"];
  review: PullRequestPage["review"];
  approvals?: number;
  mergeable?: PullRequestPage["mergeable"];
  holder?: ReviewPr["holder"];
  conversation: PullRequestEntry[];
  checkRows: PullRequestCheckRow[];
  commits: PullRequestCommit[];
  files: readonly FileIn[];
  /** Threads by what the line says. */
  threads?: readonly { id: string; path: string; match: string; resolved?: boolean; comments: readonly (readonly [author: string, ago: number, body: string])[] }[];
  /** What an agent review finds, by what the line says. */
  agent?: readonly { path: string; match: string; kind: FindingSeed["kind"]; title: string; body: string }[];
};

function build(spec: Spec): { pr: ReviewPr; agent: readonly FindingSeed[] } {
  const files = spec.files.map(fileOf);
  const key = `${spec.repository}#${spec.number}`;
  const threads: Thread[] = (spec.threads ?? []).map((one) => ({
    id: `${key}:${one.id}`,
    path: one.path,
    line: lineOf(files, one.path, one.match),
    resolved: one.resolved ?? false,
    comments: one.comments.map(([author, ago, body], at) => ({ id: `${key}:${one.id}:${at}`, author, at: minutesAgo(ago), body })),
  }));
  const page: PullRequestPage = {
    manifest_id: spec.manifestId,
    number: spec.number,
    base: "main",
    author: spec.author,
    created_at: minutesAgo(spec.opened),
    additions: files.reduce((sum, one) => sum + one.additions, 0),
    deletions: files.reduce((sum, one) => sum + one.deletions, 0),
    mergeable: spec.mergeable ?? "mergeable",
    review: spec.review,
    approvals: spec.approvals ?? 0,
    conversation: spec.conversation,
    checks: spec.checkRows,
    files: files.map(({ path, status, additions, deletions }) => ({ path, status, additions, deletions })),
    commits: spec.commits,
  };
  const pr: ReviewPr = {
    key,
    repository: spec.repository,
    manifestId: spec.manifestId,
    number: spec.number,
    title: spec.title,
    branch: spec.branch,
    address: `${FORGE}/${spec.repository}/pull/${spec.number}`,
    draft: false,
    checks: spec.checks,
    tab: spec.tab,
    ...(spec.holder === undefined ? {} : { holder: spec.holder }),
    body: spec.body,
    page,
    files,
    threads,
  };
  const agent = (spec.agent ?? []).map(({ match, ...one }) => ({ ...one, line: lineOf(files, one.path, match) }));
  return { pr, agent };
}

// ---- #1899: asked of him by marta, with threads ----------------------------------------------------------

const RUST = "crates/fleet/src/plan_usage.rs";
const MAIN_TS = "apps/desktop/src/main/plan-usage.ts";
const PROTOCOL_TS = "packages/protocol/src/plan-usage.ts";
const SOURCE_TS = "apps/desktop/src/renderer/src/fleet-plan-usage.ts";

const marta = build({
  repository: "mele-labs/armada",
  manifestId: "armada",
  number: 1899,
  title: "Cache the plan windows for a minute",
  branch: "marta/plan-windows-cache",
  author: "marta",
  tab: "requested",
  opened: 205,
  body: "The title bar's meters read the plan windows every five seconds, and each read is a call to the provider.\n\n- Fleet keeps the last read for **60 seconds** and serves it again\n- Bridge asks once a minute instead of every five seconds\n- `read_at` says when Fleet last read, so the meters can say how old they are\n\nThe windows move on the minute, so a minute is the longest a figure is wrong by.",
  checks: "passed",
  review: "required",
  conversation: [
    { kind: "comment", author: "marta", at: minutesAgo(190), body: "Asked for review from @nick: the Rust side is the part to read." },
    { kind: "review", author: "devon", at: minutesAgo(120), verdict: "commented", body: "Two small things inline. The cache itself reads right." },
  ],
  checkRows: checks("mele-labs/armada", 1899, [
    ["build", "passed", 128],
    ["test · rust", "passed", 238],
    ["test · bridge", "passed", 214],
    ["test · protocol", "passed", 71],
    ["verify-foundations", "passed", 57],
    ["typecheck", "passed", 94],
  ]),
  commits: commits([
    ["a41c0d7", "Keep the last plan read for a minute", "marta", 200],
    ["f9b2e63", "Read the plan once a minute in Bridge", "marta", 196],
    ["02d7a8c", "Say when the windows were read", "marta", 150],
  ]),
  files: [
    [
      RUST,
      "modified",
      [
        "@@ -1,16 +1,52 @@",
        " use std::time::{Duration, Instant};",
        " ",
        " use tokio::sync::Mutex;",
        " ",
        " use crate::anthropic::{self, Windows};",
        "+",
        "+/// The windows move on the minute, so a read younger than this is served again.",
        "+const CACHE_FOR: Duration = Duration::from_secs(60);",
        " ",
        "-pub async fn read(client: &anthropic::Client) -> Result<Windows, ReadError> {",
        "-    client.windows().await",
        "+struct Held {",
        "+    at: Instant,",
        "+    windows: Windows,",
        "+}",
        "+",
        "+pub struct PlanUsage {",
        "+    client: anthropic::Client,",
        "+    held: Mutex<Option<Held>>,",
        " }",
        "+",
        "+impl PlanUsage {",
        "+    pub fn new(client: anthropic::Client) -> Self {",
        "+        Self { client, held: Mutex::new(None) }",
        "+    }",
        "+",
        "+    pub async fn read(&self) -> Result<Windows, ReadError> {",
        "+        let mut held = self.held.lock().await;",
        "+        if let Some(one) = held.as_ref() {",
        "+            if one.at.elapsed() < CACHE_FOR {",
        "+                return Ok(one.windows.clone());",
        "+            }",
        "+        }",
        "+        let windows = self.client.windows().await?;",
        "+        *held = Some(Held { at: Instant::now(), windows: windows.clone() });",
        "+        Ok(windows)",
        "+    }",
        "+}",
        "+",
        "+#[cfg(test)]",
        "+mod tests {",
        "+    use super::*;",
        "+",
        "+    #[tokio::test(start_paused = true)]",
        "+    async fn a_read_inside_the_minute_is_served_again() {",
        "+        let usage = PlanUsage::new(anthropic::Client::counting());",
        "+        usage.read().await.unwrap();",
        "+        tokio::time::advance(Duration::from_secs(59)).await;",
        "+        usage.read().await.unwrap();",
        "+        assert_eq!(usage.client.calls(), 1);",
        "+    }",
        "+}",
      ],
    ],
    [
      MAIN_TS,
      "modified",
      [
        "@@ -8,15 +8,22 @@ import { fleet } from \"./fleet-link\";",
        " ",
        "-const EVERY_MS = 5_000;",
        "+const EVERY_MS = 60_000;",
        " ",
        " let last: PlanUsage | undefined;",
        "+let timer: NodeJS.Timeout | undefined;",
        " ",
        " export async function readPlanUsage(): Promise<PlanUsage | undefined> {",
        "   const answer = await fleet.get<PlanUsage>(\"/plan_usage\");",
        "-  last = answer.ok ? answer.value : last;",
        "+  if (answer.ok) last = answer.value;",
        "   return last;",
        " }",
        " ",
        " export function watchPlanUsage(tell: (usage: PlanUsage) => void): () => void {",
        "-  const timer = setInterval(() => void readPlanUsage().then((u) => u && tell(u)), EVERY_MS);",
        "-  return () => clearInterval(timer);",
        "+  timer = setInterval(() => void readPlanUsage().then((usage) => usage && tell(usage)), EVERY_MS);",
        "+  return () => clearInterval(timer);",
        " }",
      ],
    ],
    [
      PROTOCOL_TS,
      "modified",
      [
        "@@ -14,6 +14,9 @@ export type PlanWindow = {",
        " ",
        " export type PlanUsage = {",
        "   windows: PlanWindow[];",
        "+  /** RFC 3339. When Fleet last read the windows from the provider. */",
        "+  read_at: string;",
        " };",
        " ",
        " export type PlanKind = PlanWindow[\"kind\"];",
      ],
    ],
    [
      SOURCE_TS,
      "modified",
      [
        "@@ -20,10 +20,14 @@ export type PlanUsageSource = {",
        " export function createFleetPlanUsage(api: BridgeApi): PlanUsageSource {",
        "   let held: PlanUsage | undefined;",
        "-  const names = new Set<string>();",
        "+  const listeners = new Set<(usage: PlanUsage) => void>();",
        "   return {",
        "     get: () => held,",
        "-    subscribe: (name, tell) => names.add(name),",
        "+    subscribe: (tell) => {",
        "+      listeners.add(tell);",
        "+      return () => listeners.delete(tell);",
        "+    },",
        "   };",
        " }",
      ],
    ],
    [
      "crates/fleet/src/lib.rs",
      "modified",
      [
        "@@ -41,6 +41,7 @@ pub mod plan_usage;",
        " pub struct Fleet {",
        "     store: Store,",
        "+    plan_usage: plan_usage::PlanUsage,",
        "     gate: Gate,",
        " }",
      ],
    ],
    [
      "docs/concepts/plan-usage.md",
      "modified",
      [
        "@@ -9,7 +9,10 @@ Two windows count against a plan: five hours and seven days.",
        " ",
        " ## How fresh it is",
        " ",
        "-Fleet reads the windows when Bridge asks, so a figure is as old as the last ask.",
        "+Fleet keeps the last read for a minute and serves it again to every ask inside",
        "+that minute. `read_at` is when Fleet last read, so a meter can say how old its",
        "+figure is. The windows move on the minute, so a figure is never more than a",
        "+minute wrong.",
      ],
    ],
  ],
  threads: [
    {
      id: "ttl",
      path: RUST,
      match: "const CACHE_FOR",
      comments: [
        ["devon", 118, "Should this be a setting? Other windows are `settings.toml` rows."],
        ["marta", 96, "Kept as a const for now. A setting needs a registry row and the keyboard of the Settings page, and nobody has asked to change it."],
      ],
    },
    {
      id: "name",
      path: SOURCE_TS,
      match: "const listeners",
      resolved: true,
      comments: [
        ["devon", 117, "nit: `names` was a set of strings and is now a set of callbacks."],
        ["marta", 100, "Renamed."],
      ],
    },
    {
      id: "readat",
      path: PROTOCOL_TS,
      match: "read_at: string",
      comments: [["priya", 90, "Is this RFC 3339 like the other instants in this file, or epoch millis?"]],
    },
  ],
  agent: [
    {
      path: RUST,
      match: "if one.at.elapsed() < CACHE_FOR",
      kind: "bug",
      title: "A reset inside the minute is served late",
      body: "The cache is keyed on its age alone. A window that resets at 14:00:10 is still served with last minute's percentage until 14:01.\n\nCompare `resets_at` of the held windows to now, and read again once one has passed.",
    },
    {
      path: MAIN_TS,
      match: "timer = setInterval",
      kind: "risk",
      title: "A second watch loses the first interval",
      body: "`timer` is one module variable. Calling `watchPlanUsage` twice overwrites it, and the first interval can no longer be cleared.",
    },
    {
      path: PROTOCOL_TS,
      match: "read_at: string",
      kind: "question",
      title: "Is read_at required?",
      body: "A Fleet that predates this change sends none, and Bridge parses it as present. Make it optional, or bump the protocol.",
    },
  ],
});

// ---- #1903: asked of him by devon ------------------------------------------------------------------------

const devon = build({
  repository: "mele-labs/armada",
  manifestId: "armada",
  number: 1903,
  title: "Serve a bay's history from one query",
  branch: "devon/bay-history-query",
  author: "devon",
  tab: "requested",
  opened: 64,
  body: "A bay's pane asked Fleet for every slot event and filtered them in Bridge.\n\nThe query answers one bay's leases and checkouts, newest first, and says when it cut.",
  checks: "running",
  review: "none",
  conversation: [{ kind: "comment", author: "devon", at: minutesAgo(60), body: "Draft shape is in `bay-history.ts`. The Rust side reads the same rows the timeline does." }],
  checkRows: checks("mele-labs/armada", 1903, [
    ["build", "passed", 131],
    ["test · rust", "running"],
    ["test · bridge", "running"],
    ["typecheck", "passed", 97],
  ]),
  commits: commits([
    ["7bd0c19", "Answer a bay's history from one query", "devon", 62],
    ["c3a5f48", "Draw the history from the query's rows", "devon", 50],
  ]),
  files: [
    [
      "crates/fleet/src/bay_history.rs",
      "added",
      [
        "@@ -0,0 +1,34 @@",
        "+use crate::events::{Event, Events};",
        "+use crate::pool::SlotId;",
        "+",
        "+/// How many entries a bay's history carries before it says it cut.",
        "+const LIMIT: usize = 200;",
        "+",
        "+pub struct BayHistory {",
        "+    pub entries: Vec<Entry>,",
        "+    pub cut: bool,",
        "+}",
        "+",
        "+pub fn of(events: &Events, slot: SlotId) -> BayHistory {",
        "+    let mut entries: Vec<Entry> = events",
        "+        .iter()",
        "+        .filter(|one| one.detail() == Some(slot.label().as_str()))",
        "+        .filter_map(Entry::from_event)",
        "+        .collect();",
        "+    entries.sort_by(|a, b| b.at.cmp(&a.at));",
        "+    let cut = entries.len() > LIMIT;",
        "+    entries.truncate(LIMIT);",
        "+    BayHistory { entries, cut }",
        "+}",
      ],
    ],
    [
      "packages/protocol/src/bay-history.ts",
      "modified",
      [
        "@@ -3,8 +3,12 @@",
        " export type BayHistoryEntry =",
        "   | { kind: \"leased\"; at: string; holder: BayHolder; branch?: string }",
        "   | { kind: \"released\"; at: string; holder: BayHolder }",
        "   | { kind: \"checkout\"; at: string; sha: string; to: string; from?: string };",
        " ",
        "-export type BayHistory = { manifest_id: string; slot: number; entries: BayHistoryEntry[] };",
        "+export type BayHistory = {",
        "+  manifest_id: string;",
        "+  slot: number;",
        "+  entries: BayHistoryEntry[];",
        "+  /** The query stopped at its limit and older entries exist. */",
        "+  cut: boolean;",
        "+};",
      ],
    ],
    [
      "apps/desktop/src/renderer/src/cockpit/deck/bay-history.tsx",
      "modified",
      [
        "@@ -30,9 +30,11 @@ export function BayHistoryPane({ bay }: { bay: BayView }) {",
        "-  const events = useFleetEvents();",
        "-  const entries = historyOf(events, bay.slot);",
        "+  const history = useBayHistory(bay.manifest, bay.slot);",
        "+  const entries = history?.entries ?? [];",
        "   return (",
        "     <section className=\"deck-history\" aria-label=\"History\">",
        "       <Holders entries={entries} />",
        "+      {history?.cut === true ? <Cut /> : null}",
        "     </section>",
        "   );",
      ],
    ],
  ],
});

// ---- #212 in pocket: asked of him by mia ------------------------------------------------------------------

const mia = build({
  repository: "mele-labs/pocket",
  manifestId: "pocket",
  number: 212,
  title: "Bump the manifest icons",
  branch: "mia/manifest-icons",
  author: "mia",
  tab: "requested",
  opened: 120,
  body: "The home screen icons were 144 and 192. The install prompt wants 512 as well, with a maskable one.",
  checks: "passed",
  review: "required",
  conversation: [],
  checkRows: checks("mele-labs/pocket", 212, [
    ["build", "passed", 64],
    ["test", "passed", 41],
    ["lighthouse", "passed", 88],
  ]),
  commits: commits([["e1f7b02", "Add the 512 and maskable icons", "mia", 118]]),
  files: [
    [
      "public/manifest.webmanifest",
      "modified",
      [
        "@@ -6,8 +6,18 @@",
        "   \"display\": \"standalone\",",
        "   \"icons\": [",
        "     { \"src\": \"/icons/192.png\", \"sizes\": \"192x192\", \"type\": \"image/png\" },",
        "-    { \"src\": \"/icons/144.png\", \"sizes\": \"144x144\", \"type\": \"image/png\" }",
        "+    { \"src\": \"/icons/144.png\", \"sizes\": \"144x144\", \"type\": \"image/png\" },",
        "+    { \"src\": \"/icons/512.png\", \"sizes\": \"512x512\", \"type\": \"image/png\" },",
        "+    {",
        "+      \"src\": \"/icons/maskable-512.png\",",
        "+      \"sizes\": \"512x512\",",
        "+      \"type\": \"image/png\",",
        "+      \"purpose\": \"maskable\"",
        "+    }",
        "   ]",
        " }",
      ],
    ],
    [
      "src/sw/precache.ts",
      "modified",
      [
        "@@ -3,6 +3,8 @@ export const PRECACHE = [",
        "   \"/\",",
        "   \"/icons/192.png\",",
        "+  \"/icons/512.png\",",
        "+  \"/icons/maskable-512.png\",",
        "   \"/app.js\",",
        " ];",
      ],
    ],
  ],
});

// ---- #1896: his own Session's, held ------------------------------------------------------------------------

const RECONNECT = "apps/desktop/src/renderer/src/cockpit/reconnect.ts";
const LINK = "apps/desktop/src/main/fleet-link.ts";

const reconnect = build({
  repository: "mele-labs/armada",
  manifestId: "armada",
  number: 1896,
  title: "Fix the reconnect wait",
  branch: "fleet/reconnect-wait",
  author: "claude",
  tab: "mine",
  opened: 40,
  holder: { id: "f1", title: "Fix the reconnect wait" },
  body: "When Bridge loses its socket to Fleet it waited a flat 30 seconds.\n\n- the first wait is 5 seconds\n- each one after doubles, up to the 60 second cap\n- a wait is jittered by 20% so two windows do not reconnect together",
  checks: "failed",
  review: "required",
  conversation: [{ kind: "comment", author: "claude", at: minutesAgo(38), body: "The first wait is 5 seconds and doubles up to the cap. desktop_test is red on the resize case, which this does not touch." }],
  checkRows: checks("mele-labs/armada", 1896, [
    ["build", "passed", 122],
    ["test · rust", "passed", 244],
    ["desktop_test", "failed", 201],
    ["verify-foundations", "passed", 56],
    ["typecheck", "passed", 93],
  ]),
  commits: commits([
    ["5c18d3e", "Wait 5 seconds first, then back off", "claude", 38],
    ["80a2f6b", "Jitter the wait", "claude", 31],
  ]),
  files: [
    [
      RECONNECT,
      "modified",
      [
        "@@ -1,12 +1,24 @@",
        "-const FIRST_WAIT_MS = 30_000;",
        "+const FIRST_WAIT_MS = 5_000;",
        " const CAP_MS = 60_000;",
        "+const JITTER = 0.2;",
        " ",
        " /** How long to wait before reconnect number `attempt`, counting from zero. */",
        " export function waitFor(attempt: number): number {",
        "-  return FIRST_WAIT_MS;",
        "+  const grown = FIRST_WAIT_MS * 2 ** attempt;",
        "+  return Math.min(CAP_MS, grown);",
        " }",
        "+",
        "+/** A wait spread by `JITTER` either side, so two windows do not knock together. */",
        "+export function jittered(wait: number, random: () => number = Math.random): number {",
        "+  return Math.round(wait * (1 - JITTER + random() * JITTER * 2));",
        "+}",
      ],
    ],
    [
      LINK,
      "modified",
      [
        "@@ -28,14 +28,20 @@ let socket: WebSocket | undefined;",
        " export function connect(): void {",
        "   const socket = new WebSocket(url());",
        "+  socket.on(\"open\", () => {",
        "+    announce(\"connected\");",
        "+  });",
        "   socket.on(\"close\", () => {",
        "     announce(\"lost\");",
        "-    setTimeout(connect, 30_000);",
        "+    const wait = jittered(waitFor(attempt));",
        "+    attempt += 1;",
        "+    setTimeout(connect, wait);",
        "   });",
        " }",
      ],
    ],
    [
      "apps/desktop/src/renderer/src/cockpit/reconnect.test.ts",
      "added",
      [
        "@@ -0,0 +1,22 @@",
        "+import { describe, expect, it } from \"vitest\";",
        "+",
        "+import { jittered, waitFor } from \"./reconnect\";",
        "+",
        "+describe(\"waitFor\", () => {",
        "+  it(\"starts at five seconds\", () => {",
        "+    expect(waitFor(0)).toBe(5_000);",
        "+  });",
        "+",
        "+  it(\"doubles, then holds at the cap\", () => {",
        "+    expect(waitFor(1)).toBe(10_000);",
        "+    expect(waitFor(3)).toBe(40_000);",
        "+    expect(waitFor(9)).toBe(60_000);",
        "+  });",
        "+});",
        "+",
        "+describe(\"jittered\", () => {",
        "+  it(\"spreads a wait by a fifth either side\", () => {",
        "+    expect(jittered(10_000, () => 0)).toBe(8_000);",
        "+    expect(jittered(10_000, () => 1)).toBe(12_000);",
        "+  });",
        "+});",
      ],
    ],
    [
      "docs/practices/protocol.md",
      "modified",
      [
        "@@ -88,7 +88,9 @@ A lost socket is reconnected by Bridge, never by Fleet.",
        " ",
        "-The first wait is 30 seconds and stays there.",
        "+The first wait is 5 seconds. Each one after doubles up to 60 seconds, and is",
        "+spread by a fifth either side so two windows do not reconnect in step.",
      ],
    ],
  ],
  agent: [
    {
      path: LINK,
      match: "attempt += 1;",
      kind: "bug",
      title: "The attempt count is never reset",
      body: "`attempt` grows on every drop and nothing sets it back to zero when the socket opens. After the sixth drop in a day, every reconnect waits the full cap.\n\nReset it in the `open` handler.",
    },
    {
      path: RECONNECT,
      match: "return Math.round(wait *",
      kind: "risk",
      title: "Jitter can pass the cap",
      body: "The cap is applied in `waitFor`, before the spread. A 60 second wait comes out as up to 72 seconds.",
    },
  ],
});

// ---- #1892: his Job's ----------------------------------------------------------------------------------------

const pause = build({
  repository: "mele-labs/armada",
  manifestId: "armada",
  number: 1892,
  title: "Add the pause marker column",
  branch: "armada/82-pause-column",
  author: "claude",
  tab: "mine",
  opened: 30,
  body: "Adds `paused_at` to the jobs table, null until a Job is paused.",
  checks: "running",
  review: "none",
  conversation: [],
  checkRows: checks("mele-labs/armada", 1892, [
    ["build", "passed", 119],
    ["test · rust", "running"],
    ["verify-foundations", "passed", 55],
  ]),
  commits: commits([["9e4b7a0", "Add the pause marker column", "claude", 29]]),
  files: [
    [
      "crates/store/migrations/0043_pause_marker.sql",
      "added",
      [
        "@@ -0,0 +1,6 @@",
        "+-- A Job is paused when this is set, and resumed when it is cleared.",
        "+ALTER TABLE jobs ADD COLUMN paused_at TEXT;",
        "+",
        "+UPDATE jobs SET paused_at = NULL;",
        "+",
        "+CREATE INDEX jobs_paused ON jobs (paused_at) WHERE paused_at IS NOT NULL;",
      ],
    ],
    [
      "crates/store/src/jobs.rs",
      "modified",
      [
        "@@ -44,6 +44,7 @@ pub struct JobRow {",
        "     pub branch: Option<String>,",
        "     pub started_at: Option<String>,",
        "+    pub paused_at: Option<String>,",
        " }",
      ],
    ],
    [
      "packages/protocol/src/job.ts",
      "modified",
      [
        "@@ -52,6 +52,8 @@ export type JobSummary = {",
        "   started_at?: string;",
        "+  /** RFC 3339. Set while the Job is paused. */",
        "+  paused_at?: string;",
        "   branch?: string;",
        " };",
      ],
    ],
  ],
});

/** Every pull request the mock forge holds, and what an agent review of each would find. */
export function reviewSeed(): ReviewSeed {
  const all = [marta, devon, mia, reconnect, pause];
  return {
    me: "nick",
    prs: all.map((one) => one.pr),
    agent: Object.fromEntries(all.filter((one) => one.agent.length > 0).map((one) => [one.pr.key, one.agent])),
  };
}
