// What a mock forge holds of five pull requests for the review surface: three that teammates asked his review
// of, and two that agents of his opened. Each is read as what it makes happen: an explanation, then themes, and each
// theme's changes in the order they were made, with why and a real-looking hunk as the evidence. One of the three has
// threads already, so a reply and a resolve have something to act on, and each has what the review tools would find.
// **The patches are written by hand**, and a thread or a finding names its line by what the line says.
//
// Built on the helpers `session-pr-pages.ts` has for the panel's tabs, which read the same shapes.

import type { PullRequestCheckRow, PullRequestCommit, PullRequestEntry, PullRequestPage } from "@armada/protocol";
import { drawnOf } from "@armada/screens/src/drawn-patch";

import type { Account, Change, Hunk, ReviewPr, Theme, ToolDef } from "../pr-review/model";
import { ARMADA_DEFS, type FindingSeed, type MockPr, type ReviewSeed, type SeedThread } from "./review-seed";

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

type HunkIn = readonly [path: string, patch: readonly string[]];
type ChangeIn = { id: string; title: string; why: string; hunks: readonly HunkIn[] };
type ThemeIn = { id: string; title: string; purpose: string; changes: readonly ChangeIn[] };

/** A hunk as the reader draws it: the patch split into the lines git wrote. */
function hunkOf([path, patch]: HunkIn): Hunk {
  const drawn = drawnOf(`diff --git a/${path} b/${path}\n${patch.join("\n")}`, () => "");
  return { path, lines: drawn.files[0]?.lines ?? [] };
}

type Spec = {
  repository: string;
  manifestId: string;
  number: number;
  title: string;
  branch: string;
  author: string;
  lists: ReviewPr["lists"];
  team?: string;
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
  summary: string;
  explainedBy: MockPr["explainedBy"];
  themes: readonly ThemeIn[];
  /** Threads by the change they are on, and the line of its first hunk by what it says. */
  threads?: readonly { id: string; change: string; match?: string; resolved?: boolean; comments: readonly (readonly [author: string, ago: number, body: string])[] }[];
  /** What each of the other explanations says, by the kind's id. */
  accounts?: Readonly<Record<string, Account>>;
  /** What a question about it would come to, and the questions already asked of it. */
  answers?: readonly { when: string; body: string }[];
  asked?: readonly string[];
  /** What each tool finds, by the tool's id. */
  results?: Readonly<Record<string, readonly FindingSeed[]>>;
};

function build(spec: Spec): { pr: MockPr; results: Readonly<Record<string, readonly FindingSeed[]>> } {
  const themes: Theme[] = spec.themes.map((theme) => ({
    ...theme,
    changes: theme.changes.map((one): Change => ({ ...one, hunks: one.hunks.map(hunkOf) })),
  }));
  const changes = themes.flatMap((theme) => theme.changes);
  const key = `${spec.repository}#${spec.number}`;
  const threads: SeedThread[] = (spec.threads ?? []).map((one) => {
    const at = changes.find((change) => change.id === one.change);
    if (at === undefined) throw new Error(`${key} has no change ${one.change}`);
    const line = one.match === undefined ? -1 : at.hunks[0]!.lines.findIndex((text) => text.text.includes(one.match!));
    if (one.match !== undefined && line < 0) throw new Error(`${one.change} has no line holding “${one.match}”`);
    return {
      id: `${key}:${one.id}`,
      change: one.change,
      hunk: 0,
      ...(line < 0 ? {} : { line }),
      resolved: one.resolved ?? false,
      comments: one.comments.map(([author, ago, body], n) => ({ id: `${key}:${one.id}:${n}`, author, at: minutesAgo(ago), body })),
    };
  });
  const lines = changes.flatMap((change) => change.hunks.flatMap((hunk) => hunk.lines));
  const page: PullRequestPage = {
    manifest_id: spec.manifestId,
    number: spec.number,
    base: "main",
    author: spec.author,
    created_at: minutesAgo(spec.opened),
    additions: lines.filter((one) => one.kind === "added").length,
    deletions: lines.filter((one) => one.kind === "removed").length,
    mergeable: spec.mergeable ?? "mergeable",
    review: spec.review,
    approvals: spec.approvals ?? 0,
    conversation: spec.conversation,
    checks: spec.checkRows,
    files: [...new Set(changes.flatMap((change) => change.hunks.map((hunk) => hunk.path)))].map((path) => ({
      path,
      status: "modified" as const,
      additions: changes.flatMap((change) => change.hunks).filter((hunk) => hunk.path === path).flatMap((hunk) => hunk.lines).filter((one) => one.kind === "added").length,
      deletions: changes.flatMap((change) => change.hunks).filter((hunk) => hunk.path === path).flatMap((hunk) => hunk.lines).filter((one) => one.kind === "removed").length,
    })),
    commits: spec.commits,
  };
  const pr: MockPr = {
    key,
    repository: spec.repository,
    manifestId: spec.manifestId,
    number: spec.number,
    title: spec.title,
    branch: spec.branch,
    address: `${FORGE}/${spec.repository}/pull/${spec.number}`,
    draft: false,
    checks: spec.checks,
    head: spec.commits.at(-1)?.sha ?? "",
    updatedAt: minutesAgo(spec.opened),
    lists: spec.lists,
    ...(spec.team === undefined ? {} : { team: spec.team }),
    ...(spec.holder === undefined ? {} : { holder: spec.holder }),
    body: spec.body,
    page,
    explanation: { summary: spec.summary, themes },
    explainedBy: spec.explainedBy,
    accounts: spec.accounts ?? {},
    answers: spec.answers ?? [],
    asked: spec.asked ?? [],
    threads,
  };
  return { pr, results: spec.results ?? {} };
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
  lists: ["requested", "team", "assigned"],
  team: "@Mele-Labs/fleet",
  opened: 205,
  body: "## Why\n\nThe title bar's meters read the plan windows every five seconds, and each read is a call to the provider. With two windows open that is 24 calls a minute before a single Session has started, against an allowance of 30. On 11 Oct that was enough to rate limit three Sessions (#1880).\n\n## What changes\n\n- Fleet keeps the last read for **60 seconds** and serves it again\n- Bridge asks once a minute instead of every five seconds\n- `read_at` says when Fleet last read, so the meters can say how old they are\n- A failed read is not cached, so the next ask tries the provider again\n\nThe windows move on the minute, so a minute is the longest a figure is wrong by.\n\n## How it is cached\n\nThe cache is one `Held` behind a `Mutex`, filled on the first ask and replaced when it is older than `CACHE_FOR`:\n\n```rust\nconst CACHE_FOR: Duration = Duration::from_secs(60);\n\nasync fn windows(&self) -> Result<Reading, Error> {\n    let mut held = self.held.lock().await;\n    if let Some(one) = held.as_ref().filter(|one| one.at.elapsed() < CACHE_FOR) {\n        return Ok(one.reading());\n    }\n    let fresh = anthropic::windows().await?;\n    *held = Some(Held { at: Instant::now(), windows: fresh });\n    Ok(held.as_ref().unwrap().reading())\n}\n```\n\n## Not in this pull request\n\n- A shared cache across Fleets on one machine. There is only ever one Fleet per home, so it would add a file to guard for nothing.\n- Pushing the figure to Bridge. Bridge still asks; it just asks less.\n- Any change to how the meters are drawn beyond the age line.\n\n## Compatibility\n\n`read_at` is a new required field on the wire. Bridge and Fleet ship together, so the protocol version moves with it and an old Bridge is refused at connect, as for any wire change.\n\nCloses #1871.",
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
  asked: ["What happens to the cached read if Fleet restarts?"],
  answers: [
    { when: "restart", body: "It is lost. The cache is a `Held` in memory in `plan_usage.rs`, so the first read after a restart calls the provider, and the minute starts from there." },
    { when: "gh|signed out|sign.?in", body: "No. The plan windows come from the provider, not from `gh`, so a signed out `gh` does not touch this cache. The pull request panel's own reads are unchanged." },
  ],
  accounts: {
    "user-impact": {
      shape: "change",
      rows: [
        { what: "The plan meters in the title bar", before: "Each meter read the provider every five seconds, so a figure was never more than five seconds old.", after: "A figure can be up to a minute old, and the meter says how old it is." },
        { what: "A second window", before: "Every window asked the provider on its own.", after: "Windows ask Fleet once a minute, and Fleet answers from its last read." },
        { what: "A provider that is slow", before: "The meters waited on each read, and stalled with it.", after: "The last read is served again until its minute is up." },
      ],
    },
    "how-to-test": {
      shape: "steps",
      steps: [
        { do: "Run the plan usage tests, by name.", code: "armada check test plan_usage", look: "The cache cases pass: a second read inside the minute is served, and one after it calls the provider again.", from: { from: "doc", path: "crates/fleet/CLAUDE.md", address: `${FORGE}/mele-labs/armada/blob/main/crates/fleet/CLAUDE.md` } },
        { do: "Start Fleet on this branch and watch its log for plan reads.", code: "tail -f ~/.armada/fleet.log | grep plan_usage", look: "One provider call a minute, where it was one every five seconds.", from: { from: "doc", path: "crates/fleet/README.md", address: `${FORGE}/mele-labs/armada/blob/main/crates/fleet/README.md` } },
        { do: "Open a second Bridge window and leave both on the Cockpit for two minutes.", look: "Still one call a minute in the log, not two.", from: { from: "doc", path: "apps/desktop/CLAUDE.md", address: `${FORGE}/mele-labs/armada/blob/main/apps/desktop/CLAUDE.md` } },
        { do: "Hover a plan meter in the title bar.", look: "It says how long ago Fleet read the figure, counting up from read_at.", from: { from: "doc", path: "docs/practices/bridge.md", address: `${FORGE}/mele-labs/armada/blob/main/docs/practices/bridge.md` } },
      ],
    },
    why: {
      shape: "reason",
      problem: "Every open window read the plan windows every five seconds, and each read was a call to the provider. The provider allows 30 requests a minute to an account, and the meters used the same allowance a Session needs to start.",
      cost: "On the afternoon of 11 Oct two windows and three Sessions went over it, and Sessions began failing with a rate limit error before they had done anything.",
      links: [
        { label: "#1871 Plan meters hit the provider's rate limit", address: `${FORGE}/mele-labs/armada/issues/1871` },
        { label: "#1880 Sessions rate limited on 11 Oct", address: `${FORGE}/mele-labs/armada/issues/1880` },
      ],
    },
  },
  summary:
    "The title bar's meters read the plan windows every five seconds, and every read called the provider. Fleet now keeps its last read for a minute and serves it again, Bridge asks once a minute, and each reading carries the instant it was taken so a meter can say how old its figure is.",
  explainedBy: "agent",
  themes: [
    {
      id: "keep",
      title: "Fleet keeps the last read",
      purpose: "A read inside the minute is served from memory, so the provider is called once a minute.",
      changes: [
        {
          id: "hold",
          title: "Hold the last read for a minute",
          why: "Every ask called the provider. The windows move on the minute, so a read held for a minute is never more than a minute wrong.",
          hunks: [
            [
              RUST,
              [
                "@@ -1,9 +1,22 @@",
                " use std::time::{Duration, Instant};",
                " ",
                " use tokio::sync::Mutex;",
                " ",
                " use crate::anthropic::{self, Windows};",
                "+",
                "+/// The windows move on the minute, so a read younger than this is served again.",
                "+const CACHE_FOR: Duration = Duration::from_secs(60);",
                "+",
                "+struct Held {",
                "+    at: Instant,",
                "+    windows: Windows,",
                "+}",
                "+",
                "+pub struct PlanUsage {",
                "+    client: anthropic::Client,",
                "+    held: Mutex<Option<Held>>,",
                "+}",
                " ",
              ],
            ],
          ],
        },
        {
          id: "serve",
          title: "Serve it while it is fresh",
          why: "The read became a method on the struct that holds the last one. The lock is taken first, so two asks that arrive together make one call to the provider.",
          hunks: [
            [
              RUST,
              [
                "@@ -12,5 +25,18 @@ pub struct PlanUsage {",
                "-pub async fn read(client: &anthropic::Client) -> Result<Windows, ReadError> {",
                "-    client.windows().await",
                "-}",
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
              ],
            ],
          ],
        },
        {
          id: "fleet",
          title: "Fleet owns the one cache",
          why: "One `PlanUsage` on Fleet means every ask, from any window, shares the same held read.",
          hunks: [
            [
              "crates/fleet/src/lib.rs",
              ["@@ -41,6 +41,7 @@ pub mod plan_usage;", " pub struct Fleet {", "     store: Store,", "+    plan_usage: plan_usage::PlanUsage,", "     gate: Gate,", " }"],
            ],
          ],
        },
        {
          id: "test",
          title: "Test the minute",
          why: "Paused time puts a second read at 59 seconds, and the provider is called once.",
          hunks: [
            [
              RUST,
              [
                "@@ -38,0 +60,13 @@ impl PlanUsage {",
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
          ],
        },
      ],
    },
    {
      id: "poll",
      title: "Bridge asks once a minute",
      purpose: "The title bar stops asking every five seconds for a figure that cannot have changed.",
      changes: [
        {
          id: "poll",
          title: "Poll every minute",
          why: "Five seconds was shorter than the windows move. A minute matches the held read, and the interval is kept so it can be cleared.",
          hunks: [
            [
              MAIN_TS,
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
          ],
        },
        {
          id: "notify",
          title: "Tell the title bar by callback",
          why: "The source kept a set of names and told nobody. A callback lets the meters redraw the moment a read lands.",
          hunks: [
            [
              SOURCE_TS,
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
          ],
        },
      ],
    },
    {
      id: "age",
      title: "Meters say how old a figure is",
      purpose: "A held read can be a minute old, so each reading carries the instant it was taken.",
      changes: [
        {
          id: "readat",
          title: "Add read_at to the wire",
          why: "The meters need the instant of the read to say how old their figure is.",
          hunks: [
            [
              PROTOCOL_TS,
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
          ],
        },
        {
          id: "doc",
          title: "Write the minute down",
          why: "The concept page said a figure is as old as the last ask. It is now as old as the last read.",
          hunks: [
            [
              "docs/concepts/plan-usage.md",
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
        },
      ],
    },
  ],
  threads: [
    {
      id: "ttl",
      change: "hold",
      match: "const CACHE_FOR",
      comments: [
        ["devon", 118, "Should this be a setting? Other windows are `settings.toml` rows."],
        ["marta", 96, "Kept as a const for now. A setting needs a registry row and the keyboard of the Settings page, and nobody has asked to change it."],
      ],
    },
    {
      id: "name",
      change: "notify",
      match: "const listeners",
      resolved: true,
      comments: [
        ["devon", 117, "nit: `names` was a set of strings and is now a set of callbacks."],
        ["marta", 100, "Renamed."],
      ],
    },
    {
      id: "readat",
      change: "readat",
      match: "read_at: string",
      comments: [["priya", 90, "Is this RFC 3339 like the other instants in this file, or epoch millis?"]],
    },
  ],
  results: {
    risks: [
      {
        change: "poll",
        title: "A second watch loses the first interval",
        body: "`timer` is one module variable. Calling `watchPlanUsage` twice overwrites it, and the first interval can no longer be cleared.",
      },
      {
        change: "serve",
        title: "The lock is held across the provider call",
        severity: "high",
        body: "A slow read blocks every other ask until it returns, including the ones a held read could have answered.\n\nTake the lock again after the call, and let a second ask wait on the first instead.",
      },
    ],
    "possible-bugs": [
      {
        change: "serve",
        title: "A reset inside the minute is served late",
        body: "The cache is keyed on its age alone. A window that resets at 14:00:10 is still served with last minute's percentage until 14:01.\n\nCompare `resets_at` of the held windows to now, and read again once one has passed.",
      },
    ],
    "api-breaking-changes": [
      {
        change: "readat",
        title: "read_at is required on the wire",
        severity: "low",
        body: "A Fleet that predates this change sends none, and Bridge parses it as present. Make it optional, or bump the protocol.",
      },
    ],
    "test-gaps": [{ change: "test", title: "No test for a window that resets inside the minute", body: "The one test holds time still. Nothing covers a read after `resets_at` has passed." }],
  },
});

// ---- #1903: asked of him by devon ------------------------------------------------------------------------

const devon = build({
  repository: "mele-labs/armada",
  manifestId: "armada",
  number: 1903,
  title: "Serve a bay's history from one query",
  branch: "devon/bay-history-query",
  author: "devon",
  lists: ["team"],
  team: "@Mele-Labs/bridge",
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
  summary: "A bay's pane filtered every slot event in Bridge to find one bay's history. Fleet now answers that bay's leases and checkouts, newest first, and says when it cut the list. The pane draws the answer.",
  explainedBy: "agent",
  themes: [
    {
      id: "query",
      title: "Fleet answers a bay's history",
      purpose: "One query returns one bay's entries, so Bridge stops filtering the whole event log.",
      changes: [
        {
          id: "query",
          title: "Read one bay's entries",
          why: "The events already carry the slot. Filtering there keeps the query from walking anything else.",
          hunks: [
            [
              "crates/fleet/src/bay_history.rs",
              [
                "@@ -0,0 +1,20 @@",
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
          ],
        },
        {
          id: "wire",
          title: "Say when it cut",
          why: "A list that stops at its limit has to say so, or an older lease looks like it never happened.",
          hunks: [
            [
              "packages/protocol/src/bay-history.ts",
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
          ],
        },
      ],
    },
    {
      id: "pane",
      title: "The pane draws the answer",
      purpose: "The history pane reads the query's rows and no longer sorts events itself.",
      changes: [
        {
          id: "pane",
          title: "Draw the history from the query",
          why: "The pane took every event and filtered them. It now asks for one bay and draws what comes back, with a note where the list was cut.",
          hunks: [
            [
              "apps/desktop/src/renderer/src/cockpit/deck/bay-history.tsx",
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
        },
      ],
    },
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
  lists: ["requested"],
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
  summary: "The install prompt wants a 512 pixel icon and a maskable one, and the manifest only named 144 and 192. Both are added to the manifest and to the files the service worker caches.",
  explainedBy: "agent",
  // Risks reads this one and finds nothing: the tool's empty picture, with what was checked under it.
  results: { risks: [] },
  themes: [
    {
      id: "icons",
      title: "The install prompt gets its icons",
      purpose: "The manifest names the two sizes the prompt asks for, and the app works offline with them.",
      changes: [
        {
          id: "manifest",
          title: "Name the 512 and maskable icons",
          why: "The prompt skips an app whose manifest has no 512 pixel icon.",
          hunks: [
            [
              "public/manifest.webmanifest",
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
          ],
        },
        {
          id: "precache",
          title: "Cache them for offline",
          why: "An icon not in the precache list is fetched on first use, and the install prompt can ask before it is online.",
          hunks: [
            ["src/sw/precache.ts", ["@@ -3,6 +3,8 @@ export const PRECACHE = [", "   \"/\",", "   \"/icons/192.png\",", "+  \"/icons/512.png\",", "+  \"/icons/maskable-512.png\",", "   \"/app.js\",", " ];"]],
          ],
        },
      ],
    },
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
  lists: ["mine"],
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
  accounts: {
    "user-impact": {
      shape: "change",
      rows: [
        { what: "Fleet restarts", before: "Every window stayed dark for 30 seconds.", after: "The first retry is at 5 seconds, so a restart is back in about that." },
        { what: "Fleet is away for a while", before: "A retry every 30 seconds for as long as it stayed away.", after: "Waits of 5, 10, 20 and 40 seconds, then one every 60." },
        { what: "Two windows open", before: "Both retried on the same beat.", after: "Each wait is spread by a fifth, so they do not knock together." },
      ],
    },
    "how-to-test": {
      shape: "steps",
      steps: [
        { do: "Open Bridge, then stop Fleet.", code: "armada fleet stop", look: "The link mark says reconnecting, and the first retry comes at about 5 seconds." },
        { do: "Leave Fleet stopped for two minutes.", look: "Retries at about 5, 10, 20 and 40 seconds, then every 60." },
        { do: "Start Fleet again.", code: "armada fleet start", look: "Bridge connects at its next retry and the link mark says connected." },
        { do: "Open a second window and stop Fleet again.", look: "The two windows do not retry in the same second." },
      ],
    },
    why: {
      shape: "reason",
      problem: "A Fleet restart left every Bridge window dark for 30 seconds, because the wait after a lost socket was a flat 30 seconds and the first retry was already late. A Fleet that stayed away was asked every 30 seconds for as long as it did.",
      cost: "Jobs kept progressing, but nothing could be reviewed or answered for half a minute after every update, and two windows retried in step.",
      links: [{ label: "#1859 Bridge takes 30 seconds to come back after an update", address: `${FORGE}/mele-labs/armada/issues/1859` }],
    },
  },
  summary:
    "Bridge waited a flat 30 seconds after losing its socket to Fleet, so a Fleet restart left every window dark for half a minute. The wait now starts at 5 seconds and doubles up to 60. Each wait is spread by a fifth so two windows do not knock together, and the link says when it is connected.",
  explainedBy: "session",
  themes: [
    {
      id: "back",
      title: "A dropped socket waits longer each time",
      purpose: "The first retry is quick, and a Fleet that stays away is asked less and less often.",
      changes: [
        {
          id: "back",
          title: "Back off by doubling",
          why: "A flat 30 seconds was too slow for a restart and too eager for an outage. Five seconds first, doubled each time, held at 60.",
          hunks: [
            [
              RECONNECT,
              [
                "@@ -1,9 +1,10 @@",
                "-const FIRST_WAIT_MS = 30_000;",
                "+const FIRST_WAIT_MS = 5_000;",
                " const CAP_MS = 60_000;",
                " ",
                " /** How long to wait before reconnect number `attempt`, counting from zero. */",
                " export function waitFor(attempt: number): number {",
                "-  return FIRST_WAIT_MS;",
                "+  const grown = FIRST_WAIT_MS * 2 ** attempt;",
                "+  return Math.min(CAP_MS, grown);",
                " }",
              ],
            ],
          ],
        },
        {
          id: "link",
          title: "Wait that long in the link",
          why: "The link slept a constant. It now counts its attempts and asks `waitFor` how long.",
          hunks: [
            [
              LINK,
              ["@@ -1,4 +1,5 @@", " import WebSocket from \"ws\";", "+import { jittered, waitFor } from \"../renderer/src/cockpit/reconnect\";", " ", " import { announce } from \"./status\";"],
            ],
            [
              LINK,
              [
                "@@ -28,10 +29,14 @@ import { announce } from \"./status\";",
                " let socket: WebSocket | undefined;",
                "+let attempt = 0;",
                " ",
                " export function connect(): void {",
                "   const socket = new WebSocket(url());",
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
          ],
        },
        {
          id: "test-back",
          title: "Test the back off",
          why: "The first wait, the doubling and the cap are the three numbers a later edit would break.",
          hunks: [
            [
              "apps/desktop/src/renderer/src/cockpit/reconnect.test.ts",
              [
                "@@ -0,0 +1,13 @@",
                "+import { describe, expect, it } from \"vitest\";",
                "+",
                "+import { waitFor } from \"./reconnect\";",
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
              ],
            ],
          ],
        },
        {
          id: "doc",
          title: "Write the waits down",
          why: "The protocol practice stated the flat 30 seconds, and the next person to touch the link reads it first.",
          hunks: [
            [
              "docs/practices/protocol.md",
              [
                "@@ -88,7 +88,9 @@ A lost socket is reconnected by Bridge, never by Fleet.",
                " ",
                "-The first wait is 30 seconds and stays there.",
                "+The first wait is 5 seconds. Each one after doubles up to 60 seconds, and is",
                "+spread by a fifth either side so two windows do not reconnect in step.",
              ],
            ],
          ],
        },
      ],
    },
    {
      id: "spread",
      title: "Two windows do not reconnect in step",
      purpose: "Every window that lost Fleet at once would retry at once. A spread breaks the knock.",
      changes: [
        {
          id: "jitter",
          title: "Spread each wait",
          why: "Windows that lose the socket together would all retry on the same second. A fifth either side spreads them out.",
          hunks: [
            [
              RECONNECT,
              [
                "@@ -10,3 +11,8 @@ export function waitFor(attempt: number): number {",
                "   return Math.min(CAP_MS, grown);",
                " }",
                "+",
                "+const JITTER = 0.2;",
                "+",
                "+/** A wait spread by `JITTER` either side, so two windows do not knock together. */",
                "+export function jittered(wait: number, random: () => number = Math.random): number {",
                "+  return Math.round(wait * (1 - JITTER + random() * JITTER * 2));",
                "+}",
              ],
            ],
          ],
        },
        {
          id: "test-jitter",
          title: "Test the spread",
          why: "The random source is a parameter so the two ends of the spread can be asserted exactly.",
          hunks: [
            [
              "apps/desktop/src/renderer/src/cockpit/reconnect.test.ts",
              [
                "@@ -14,0 +15,8 @@ describe(\"waitFor\", () => {",
                "+describe(\"jittered\", () => {",
                "+  it(\"spreads a wait by a fifth either side\", () => {",
                "+    expect(jittered(10_000, () => 0)).toBe(8_000);",
                "+    expect(jittered(10_000, () => 1)).toBe(12_000);",
                "+  });",
                "+});",
              ],
            ],
          ],
        },
      ],
    },
    {
      id: "connected",
      title: "Bridge says when it is connected",
      purpose: "The link announced only a loss. It now announces the open, and the title bar has words for it.",
      changes: [
        {
          id: "open",
          title: "Announce the open",
          why: "Nothing told the window the socket was back, so it stayed on its lost state until the next message.",
          hunks: [
            [
              LINK,
              [
                "@@ -31,6 +32,9 @@ export function connect(): void {",
                "   const socket = new WebSocket(url());",
                "+  socket.on(\"open\", () => {",
                "+    announce(\"connected\");",
                "+  });",
                "   socket.on(\"close\", () => {",
                "     announce(\"lost\");",
              ],
            ],
          ],
        },
        {
          id: "state",
          title: "Give the state its words",
          why: "The title bar switches on the link state, and a new state with no case draws nothing.",
          hunks: [
            [
              "apps/desktop/src/renderer/src/cockpit/link-state.ts",
              [
                "@@ -1,10 +1,12 @@",
                "-export type LinkState = \"lost\" | \"connecting\";",
                "+export type LinkState = \"connected\" | \"lost\" | \"connecting\";",
                " ",
                " export function wordsOf(state: LinkState): string {",
                "   switch (state) {",
                "+    case \"connected\":",
                "+      return \"Connected\";",
                "     case \"lost\":",
                "       return \"Lost\";",
              ],
            ],
          ],
        },
      ],
    },
  ],
  results: {
    risks: [{ change: "jitter", title: "Jitter can pass the cap", body: "The cap is applied in `waitFor`, before the spread. A 60 second wait comes out as up to 72 seconds." }],
    "possible-bugs": [
      {
        change: "link",
        title: "The attempt count is never reset",
        severity: "high",
        body: "`attempt` grows on every drop and nothing sets it back to zero when the socket opens. After the sixth drop in a day, every reconnect waits the full cap.\n\nReset it in the `open` handler.",
      },
    ],
    "api-breaking-changes": [
      {
        change: "state",
        title: "LinkState gains a member",
        severity: "low",
        body: "A window built before this change switches on `LinkState` with no default, and draws nothing for `connected`.",
      },
    ],
    "test-gaps": [{ change: "open", title: "No test for the open handler", body: "Nothing asserts that `announce` is called with `connected`, or that the attempt count is touched when the socket opens." }],
  },
});

// ---- #1892: his Job's ----------------------------------------------------------------------------------------

const pause = build({
  repository: "mele-labs/armada",
  manifestId: "armada",
  number: 1892,
  title: "Add the pause marker column",
  branch: "armada/82-pause-column",
  author: "claude",
  lists: ["mine"],
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
  summary: "A Job is paused when `paused_at` is set and resumed when it is cleared. The column is added to the jobs table, read into the store's row, and sent on the wire.",
  explainedBy: "agent",
  themes: [
    {
      id: "marker",
      title: "A Job can be marked paused",
      purpose: "One nullable column says whether a Job is paused and since when.",
      changes: [
        {
          id: "migration",
          title: "Add the column",
          why: "Null until a Job is paused, so every existing row stays running.",
          hunks: [
            [
              "crates/store/migrations/0043_pause_marker.sql",
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
          ],
        },
        {
          id: "row",
          title: "Read it into the row",
          why: "The store hands out `JobRow`, so the column has to be a field of it.",
          hunks: [
            ["crates/store/src/jobs.rs", ["@@ -44,6 +44,7 @@ pub struct JobRow {", "     pub branch: Option<String>,", "     pub started_at: Option<String>,", "+    pub paused_at: Option<String>,", " }"]],
          ],
        },
        {
          id: "wire",
          title: "Send it on the wire",
          why: "Bridge draws a paused Job from the summary, so the field has to be in it.",
          hunks: [
            [
              "packages/protocol/src/job.ts",
              ["@@ -52,6 +52,8 @@ export type JobSummary = {", "   started_at?: string;", "+  /** RFC 3339. Set while the Job is paused. */", "+  paused_at?: string;", "   branch?: string;", " };"],
            ],
          ],
        },
      ],
    },
  ],
  results: {
    "migration-safety": [
      {
        change: "migration",
        title: "The UPDATE rewrites every row",
        body: "A new nullable column is already null. `UPDATE jobs SET paused_at = NULL` touches every row for no change, and holds the write lock while it does.\n\nDrop the statement.",
      },
    ],
  },
});

// ---- Found by a find, on neither of his lists ------------------------------------------------------------

const quick = (spec: { accounts?: Readonly<Record<string, Account>>; lists: ReviewPr["lists"]; team?: string; number: number; title: string; branch: string; author: string; opened: number; body: string; summary: string; theme: ThemeIn }) =>
  build({
    repository: "mele-labs/armada",
    manifestId: "armada",
    number: spec.number,
    title: spec.title,
    branch: spec.branch,
    author: spec.author,
    lists: spec.lists,
    ...(spec.team === undefined ? {} : { team: spec.team }),
    opened: spec.opened,
    body: spec.body,
    checks: "passed",
    review: "none",
    conversation: [],
    checkRows: checks("mele-labs/armada", spec.number, [
      ["build", "passed", 120],
      ["test · rust", "passed", 230],
    ]),
    commits: commits([[`${spec.number.toString(16)}a01c3`.slice(0, 7), spec.title, spec.author, spec.opened - 3]]),
    summary: spec.summary,
    explainedBy: "agent",
    ...(spec.accounts === undefined ? {} : { accounts: spec.accounts }),
    themes: [spec.theme],
  });

const hub = quick({
  lists: ["team"],
  team: "@Mele-Labs/fleet",
  number: 1907,
  title: "Name the hub's ports",
  branch: "priya/hub-ports",
  author: "priya",
  opened: 300,
  body: "## Problem\n\nThe hub bound whatever port the operating system gave it and logged it once. A restart on a new port left every window reading the old one, so each had to be closed and reopened to find the hub again. #1862 has the reports.\n\n## Change\n\nThe hub now asks for the port in the runtime file first, and falls back to a free one only if it is taken.\n\n1. Read `hub.port` from the runtime file at start\n2. Try to bind it\n3. If it is taken, bind port 0, as before, and write the port it got\n4. Write the port back before accepting a connection, so a window that reads the file never sees a port nobody listens on\n\n```rust\nlet wanted = runtime::read().ok().and_then(|file| file.hub_port);\nlet listener = match wanted {\n    Some(port) => TcpListener::bind((LOCALHOST, port)).or_else(|_| TcpListener::bind((LOCALHOST, 0)))?,\n    None => TcpListener::bind((LOCALHOST, 0))?,\n};\nruntime::write_hub_port(listener.local_addr()?.port())?;\n```\n\n## Behaviour\n\n| Case | Before | After |\n|---|---|---|\n| Restart, port free | new port | same port |\n| Restart, port taken | new port | new port, written to the file |\n| First start | random port | random port, written to the file |\n\n## Notes\n\n- The runtime file is still written atomically, with a rename, so a reader never sees half of it\n- Nothing reads `hub.port` from anywhere else; the Drones are told it on their command line\n- A Drone started before a restart still holds the old port. That is #1866 and is not touched here\n\n## Testing\n\n1. Restart Fleet with a window open and check it reconnects without a reload\n2. Run `armada check test hub_port`, which covers the three cases in the table",
  accounts: {
    "user-impact": {
      shape: "change",
      rows: [
        { what: "Restarting Fleet", before: "Every open window kept reading the old port and had to be closed and reopened.", after: "The hub comes back on the same port, so a window reconnects on its own." },
        { what: "A port that is taken", before: "Not handled; the hub took whatever it was given.", after: "The hub binds a free port and writes it to the runtime file." },
      ],
    },
    "how-to-test": {
      shape: "steps",
      steps: [
        { do: "Restart Fleet with a window open.", code: "armada fleet restart", look: "The window reconnects without a reload.", from: { from: "description" } },
        { do: "Run the hub port tests, by name.", code: "armada check test hub_port", look: "Three cases pass: free, taken, and first start.", from: { from: "description" } },
        { do: "Occupy the port, restart Fleet, and read the runtime file.", code: "nc -l 4219 &\ncat ~/.armada/runtime.json", look: "A different hub_port than before, and the window still reconnects.", from: { from: "doc", path: "crates/fleet/README.md", address: `${FORGE}/mele-labs/armada/blob/main/crates/fleet/README.md` } },
      ],
    },
    why: {
      shape: "reason",
      problem: "A restart left the hub on a new port, and every window that survived it was reading the old one.",
      cost: "A window had to be closed and reopened after every Fleet restart before it found the hub again.",
      links: [{ label: "#1862 Windows lose the hub after a Fleet restart", address: `${FORGE}/mele-labs/armada/issues/1862` }],
    },
  },
  summary: "The hub took a random port on every start, so a window that survived a restart read a stale one. It now asks for the port in the runtime file first.",
  theme: {
    id: "ports",
    title: "The hub keeps its port",
    purpose: "A restart lands on the port the runtime file names, so a window reconnects without reading anything new.",
    changes: [
      {
        id: "bind",
        title: "Ask for the recorded port",
        why: "The runtime file already says which port the last hub used. Asking for it first keeps windows connected across a restart.",
        hunks: [
          [
            "crates/fleet/src/main_hub.rs",
            [
              "@@ -52,8 +52,14 @@ pub async fn bind(runtime: &Runtime) -> io::Result<TcpListener> {",
              "-    TcpListener::bind((LOCALHOST, 0)).await",
              "+    let wanted = runtime.port().unwrap_or(0);",
              "+    match TcpListener::bind((LOCALHOST, wanted)).await {",
              "+        Ok(listener) => Ok(listener),",
              "+        Err(_) if wanted != 0 => TcpListener::bind((LOCALHOST, 0)).await,",
              "+        Err(error) => Err(error),",
              "+    }",
              " }",
            ],
          ],
        ],
      },
      {
        id: "record",
        title: "Write the port back",
        why: "A fallback port has to be the one recorded, or the next start asks for the old one again.",
        hunks: [
          ["crates/fleet/src/main_hub.rs", ["@@ -80,6 +86,7 @@ pub async fn serve(runtime: &Runtime) -> io::Result<()> {", "     let listener = bind(runtime).await?;", "+    runtime.record_port(listener.local_addr()?.port())?;", "     accept(listener).await"]],
        ],
      },
    ],
  },
});

const totals = quick({
  lists: [],
  number: 1905,
  title: "Round invoice totals once",
  branch: "marek/invoice-rounding",
  author: "marek",
  opened: 410,
  body: "Line items were rounded one by one and then summed, so an invoice could be a cent off the sum of its lines.\n\nTotals are now summed in minor units and rounded once at the end.",
  summary: "An invoice total was the sum of rounded lines, which drifts by a cent. It is now the rounded sum of the lines.",
  theme: {
    id: "rounding",
    title: "A total is rounded once",
    purpose: "The total comes from unrounded lines, so it matches what a customer adds up.",
    changes: [
      {
        id: "sum",
        title: "Sum before rounding",
        why: "Rounding each line first loses up to half a cent a line.",
        hunks: [
          [
            "services/billing/src/totals.rs",
            [
              "@@ -14,9 +14,8 @@ pub fn total(lines: &[Line]) -> Money {",
              "-    lines.iter().map(|line| line.amount().round()).sum()",
              "+    let sum: Decimal = lines.iter().map(|line| line.amount()).sum();",
              "+    Money::from_minor(sum.round_dp(2).mantissa())",
              " }",
            ],
          ],
        ],
      },
    ],
  },
});

const webhook = quick({
  lists: ["assigned"],
  number: 1908,
  title: "Retry the webhook on a 5xx",
  branch: "devon/webhook-retry",
  author: "devon",
  opened: 180,
  body: "A customer's endpoint that answered 502 once was never told about the payment.\n\nThe billing service now retries a webhook on any 5xx, three times, with a growing wait.",
  summary: "A webhook that got a 5xx was dropped. The billing service now tries it three more times with a growing wait.",
  theme: {
    id: "retry",
    title: "A webhook that failed is tried again",
    purpose: "A customer's endpoint that blinks does not miss a payment.",
    changes: [
      {
        id: "retry",
        title: "Retry on a server error",
        why: "A 5xx says the endpoint failed, not that the payment did not happen.",
        hunks: [
          [
            "services/billing/src/webhook.rs",
            [
              "@@ -31,6 +31,16 @@ pub async fn deliver(hook: &Hook, event: &Event) -> Result<(), Undelivered> {",
              "-    send(hook, event).await",
              "+    let mut wait = Duration::from_secs(2);",
              "+    for attempt in 0..=RETRIES {",
              "+        match send(hook, event).await {",
              "+            Err(Failure::Server(_)) if attempt < RETRIES => sleep(wait).await,",
              "+            other => return other.map_err(Undelivered::from),",
              "+        }",
              "+        wait *= 2;",
              "+    }",
              "+    unreachable!()",
              " }",
            ],
          ],
        ],
      },
    ],
  },
});

// ---- The tools ------------------------------------------------------------------------------------------------

const kitTools: ToolDef[] = [
  { id: "test-gaps", kind: "tool", name: "Test gaps", source: "kit", mark: "custom", agent: true, checks: "What the change does that no test would notice if it broke." },
];

/** `.armada/review-tools/` in the armada repository: one that is its own, and one that replaces the Kit's of the same name. */
const repositoryTools: Record<string, ToolDef[]> = {
  armada: [
    { id: "test-gaps", kind: "tool", name: "Test gaps", source: "repository", mark: "custom", agent: true, checks: "What the change does that no test in this repository's suites would notice if it broke. Name the suite that should hold it." },
    { id: "migration-safety", kind: "tool", name: "Migration safety", source: "repository", mark: "custom", agent: true, checks: "A migration under crates/store/migrations that locks a table, rewrites rows, or cannot be run twice." },
  ],
};

/** Every pull request the mock forge holds, the tools a repository has, and what each tool finds on each pull request. */
export function reviewSeed(): ReviewSeed {
  const all = [marta, devon, mia, reconnect, pause, hub, totals, webhook];
  return {
    me: "nick",
    // Listed newest first, as the forge lists them: the order they were written in.
    prs: all.map((one, at) => ({ ...one.pr, updatedAt: minutesAgo(at + 1) })),
    tools: {
      armada: ARMADA_DEFS,
      kit: kitTools,
      repository: repositoryTools,
      leftOut: { armada: [{ file: "/Users/nick/Development/armada/.armada/review-tools/notes.md", source: "repository", said: "No kind in its front matter" }] },
    },
    results: Object.fromEntries(all.map((one) => [one.pr.key, one.results])),
    pushes: {
      [reconnect.pr.key]: { sha: "d41e7a9", headline: "Reset the attempt count when the socket opens", author: "claude" },
    },
    finds: [
      {
        when: "touch.*files.*touched|files i.?ve (touched|changed)",
        hits: [
          { key: hub.pr.key, reason: "touches crates/fleet/src/main_hub.rs, which you changed on 14 Oct" },
          { key: marta.pr.key, reason: "touches crates/fleet/src/lib.rs, which you changed on 14 Oct" },
          { key: devon.pr.key, reason: "touches apps/desktop/src/renderer/src/cockpit/deck/bay-history.tsx, which you changed on 9 Oct" },
        ],
      },
      {
        when: "billing",
        hits: [
          { key: webhook.pr.key, reason: "touches services/billing/src/webhook.rs" },
          { key: totals.pr.key, reason: "touches services/billing/src/totals.rs" },
        ],
      },
    ],
  };
}
