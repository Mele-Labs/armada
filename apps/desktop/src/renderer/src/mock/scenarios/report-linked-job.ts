// A reported bug and the Job started on it. Three reports in Armada's Reported bugs band: one with a Job working on it, one
// fixed because that Job's pull request merged, one with no Job. The Job holds a `report` row, which is how Bridge finds a
// report's Job. The walk `report-linked-job` plays it.

import type { RepositorySummary } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";
import type { Session } from "@armada/screens/src/draft/sessions";

import type { ReportedBug, TicketRead } from "../../issues-tab/model";
import { holding } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";
import type { IssuesSeed } from "../slices/issues";

const ARMADA: RepositorySummary = {
  root: "/Users/user/code/armada",
  records_root: "/records/armada",
  manifest: { ...repository().manifest!, id: "armada", repository: "armada", path: "armada/armada.yml" },
};

const ago = (hours: number) => new Date(Date.now() - hours * 3_600_000).toISOString();
const now = () => new Date().toISOString();
const say = (id: string, text: string) => ({ id, at: now(), kind: "message", from: { kind: "agent" }, text }) as const;
const ask = (id: string, text: string) => ({ id, at: now(), kind: "message", from: { kind: "you" }, text }) as const;

const TICKETS: TicketRead[] = [
  { reference: "#2961", title: "The Cockpit list drops a row while a Job is moving", url: "https://github.com/acme/armada/issues/2961", state: "open", author: "nick", assignees: [], updated_at: ago(3), fields: [{ name: "Labels", values: ["bug", "bridge"] }], body: "A row flickers out of the list for one frame when its Job changes repository.", excerpt: "A row flickers out of the list for one frame when its Job changes repository.", linked_pull_request: false, linked_pull_requests: [], holders: [] },
  { reference: "#2940", title: "Settings, Keyboard lists a key twice", url: "https://github.com/acme/armada/issues/2940", state: "closed", author: "nick", assignees: [], updated_at: ago(60), fields: [{ name: "Labels", values: ["bug"] }], body: "Two acts show the same key in the Keyboard list.", excerpt: "Two acts show the same key in the Keyboard list.", linked_pull_request: false, linked_pull_requests: [], holders: [] },
];

const REPORTER = "01LINKEDREPORTERAAAAAAAAA";
const FIXING = "01LINKEDFIXINGBBBBBBBBBBB";
const FIXED = "01LINKEDFIXEDCCCCCCCCCCCC";

const WATCH = "report-watch-live";
const ICON = "report-sketch-icon";
const VERB = "report-waiting-verb";

const SESSIONS = (): Session[] => [
  {
    id: REPORTER,
    title: "Draw the dispatch retry as a sketch",
    manifestId: "armada",
    turn: { state: "idle" },
    lastTurnAt: ago(5),
    standing: "idle",
    rows: [ask("r1-u", "Draw how a refused dispatch is retried."), say("r1-a", "The sketch is up. Three tools gave me trouble; I reported them.")],
    attachments: [],
  },
  {
    id: FIXING,
    title: "Stop a watch once its pull request merges",
    manifestId: "armada",
    turn: { state: "working" },
    lastTurnAt: now(),
    standing: "working",
    rows: [ask("f1-u", "Fix the reported watch that stays live."), say("f1-a", "Reading how Fleet ends a watch.")],
    attachments: [{ kind: "report", id: WATCH, title: "A watch stayed live after its pull request merged" }],
  },
  {
    id: FIXED,
    title: "Accept every icon the sketch list names",
    manifestId: "armada",
    turn: { state: "idle" },
    lastTurnAt: ago(2),
    standing: "done",
    done: { since: ago(2) },
    rows: [ask("x1-u", "Fix the reported icon refusal."), say("x1-a", "The icon list and the check now read one table. The pull request merged.")],
    attachments: [
      { kind: "report", id: ICON, title: "show_sketch refused an icon the list allows" },
      { kind: "pull_request", number: 3071, title: "Read one icon table in show_sketch", branch: "fix/sketch-icon-table", address: "https://github.com/acme/armada/pull/3071", checks: { state: "passed" }, state: "merged", auto: false },
    ],
  },
];

const REPORTS = (): ReportedBug[] => [
  {
    id: WATCH,
    session_id: REPORTER,
    job_title: "Draw the dispatch retry as a sketch",
    title: "A watch stayed live after its pull request merged",
    what: "A watch started on a deploy kept running after the pull request it belonged to merged into main. all_done was refused until the watch was stopped by hand.",
    steps: ["Start a watch on a deploy with start_watch.", "Merge the pull request that deploy belongs to.", "Call list_watches. The watch is still live."],
    errors: ["fleet.watch_live: a live watch holds this Job open"],
    logs: [],
    at: ago(1),
    kept: true,
  },
  {
    id: ICON,
    session_id: REPORTER,
    job_title: "Draw the dispatch retry as a sketch",
    title: "show_sketch refused an icon the list allows",
    what: "A scene naming the icon git-branch was refused as unknown, though the icon list in the tool's description includes it.",
    steps: ["Call show_sketch with a node whose icon is git-branch.", "Read the refusal."],
    errors: ["sketch.icon_unknown: git-branch is not in the icon list"],
    logs: [".armada/scratch/scene-retry.json"],
    at: ago(26),
    kept: true,
    fixed: { at: ago(2), pull_request: 3071, title: "Read one icon table in show_sketch", url: "https://github.com/acme/armada/pull/3071" },
  },
  {
    id: VERB,
    session_id: REPORTER,
    job_title: "Draw the dispatch retry as a sketch",
    title: "waiting_for refused an item whose text began with a verb",
    what: "A list was refused because one item read Approve the plan, although its act was a valid answer with two options.",
    steps: ["Call waiting_for with one item: text Approve the plan, act kind answer, two options.", "Read the refusal."],
    errors: ["fleet.waiting_item_refused: item plan reads as an action"],
    logs: [],
    at: ago(8),
    kept: true,
  },
];

const SEED: IssuesSeed = [
  {
    tracker: { manifestId: "armada", state: "read", name: "GitHub", open: "is:open", assigned: "is:open assignee:@me", closed: "is:closed", saved: [] },
    tickets: TICKETS,
    me: "nick",
    language: "terms",
  },
];

function build(): Scenario {
  const base = holding("report-linked-job", "A reported bug and the Job started on it: its row on the Issues tab, the reader, the Job's Issues, and a fixed report under Closed", [ARMADA]);
  return {
    ...base,
    state: { ...base.state, repository: null },
    draft: { sessions: (_control, more = []) => sessionsStore([...SESSIONS(), ...more], [], new Map(), []), issuesTab: SEED, reportedBugs: REPORTS() },
  };
}

export const reportLinkedJob: Scenario = build();
