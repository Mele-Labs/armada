// Bugs agents reported in Armada itself. The Issues tab on All has a Reported bugs band over the repositories' issues: three
// reports from two Jobs, two unread and one kept, and the Cockpit marks both Jobs. Armada and storefront each read their
// issues from a tracker, so the band stands over two lists. The walk `reported-bugs` plays it.

import type { RepositorySummary } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";
import type { Session } from "@armada/screens/src/draft/sessions";

import type { ReportedBug, TicketField, TicketRead } from "../../issues-tab/model";
import { holding } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";
import type { IssuesSeed } from "../slices/issues";

const served = (id: string): RepositorySummary => ({
  root: `/Users/user/code/${id}`,
  records_root: `/records/${id}`,
  manifest: { ...repository().manifest!, id, repository: id, path: `${id}/armada.yml` },
});
const ARMADA = served("armada");
const STOREFRONT = served("storefront");

const ago = (hours: number) => new Date(Date.now() - hours * 3_600_000).toISOString();
const f = (name: string, ...values: string[]): TicketField => ({ name, values });
const ticket = (one: Omit<TicketRead, "updated_at" | "linked_pull_request" | "linked_pull_requests" | "excerpt" | "holders"> & { hours: number }): TicketRead => {
  const { hours, ...rest } = one;
  return { ...rest, updated_at: ago(hours), excerpt: rest.body.split("\n")[0] ?? "", linked_pull_request: false, linked_pull_requests: [], holders: [] };
};

const ARMADA_TICKETS: TicketRead[] = [
  ticket({ reference: "#2961", title: "The Cockpit list drops a row while a Job is moving", url: "https://github.com/acme/armada/issues/2961", state: "open", author: "nick", assignees: [], hours: 3, fields: [f("Labels", "bug", "bridge")], body: "A row flickers out of the list for one frame when its Job changes repository." }),
  ticket({ reference: "#2958", title: "Settings, Keyboard lists a key twice", url: "https://github.com/acme/armada/issues/2958", state: "open", author: "nick", assignees: [], hours: 20, fields: [f("Labels", "bug")], body: "Two acts show the same key in the Keyboard list." }),
];
const STOREFRONT_TICKETS: TicketRead[] = [
  ticket({ reference: "#2810", title: "Coupon field accepts an expired code", url: "https://github.com/acme/storefront/issues/2810", state: "open", author: "dana", assignees: [], hours: 2, fields: [f("Labels", "bug", "checkout")], body: "A code past its end date is still taken at checkout." }),
];

const now = () => new Date().toISOString();
const say = (id: string, text: string) => ({ id, at: now(), kind: "message", from: { kind: "agent" }, text }) as const;
const ask = (id: string, text: string) => ({ id, at: now(), kind: "message", from: { kind: "you" }, text }) as const;

const WATCHES = "01REPORTWATCHESAAAAAAAAAAA";
const SKETCH = "01REPORTSKETCHBBBBBBBBBBBB";

const SESSIONS = (): Session[] => [
  {
    id: WATCHES,
    title: "Stop watches when their pull request merges",
    manifestId: "armada",
    turn: { state: "working" },
    lastTurnAt: now(),
    standing: "working",
    rows: [ask("w1-u", "Stop a watch once the pull request it follows has merged."), say("w1-a", "Reading how Fleet ends a watch. Two things did not work as the tool descriptions say; I reported both.")],
    attachments: [],
  },
  {
    id: SKETCH,
    title: "Draw the dispatch retry as a sketch",
    manifestId: "armada",
    turn: { state: "idle" },
    lastTurnAt: ago(5),
    standing: "idle",
    rows: [ask("s1-u", "Draw how a refused dispatch is retried."), say("s1-a", "The sketch is up. One icon was refused, so I used a neighbour and reported it.")],
    attachments: [],
  },
];

const REPORTS = (): ReportedBug[] => [
  {
    id: "report-watch-live",
    session_id: WATCHES,
    job_title: "Stop watches when their pull request merges",
    title: "A watch stayed live after its pull request merged",
    what: "A watch started on a deploy kept running after the pull request it belonged to merged into main. Fleet showed the Job as held open by the watch, and all_done was refused until the watch was stopped by hand.",
    steps: ["Start a watch on a deploy with start_watch.", "Merge the pull request that deploy belongs to.", "Call list_watches. The watch is still live.", "Call all_done. It is refused because a live watch holds the Job open."],
    errors: ["fleet.watch_live: a live watch holds this Job open", "all_done refused: 1 watch live (qa-deploy)"],
    logs: ["~/Library/Application Support/Armada/logs/fleet.log", ".armada/scratch/qa-deploy-watch.log"],
    at: new Date(Date.now() - 20 * 60_000).toISOString(),
    kept: false,
  },
  {
    id: "report-waiting-verb",
    session_id: WATCHES,
    job_title: "Stop watches when their pull request merges",
    title: "waiting_for refused an item whose text began with a verb",
    what: "A list was refused because one item read Approve the plan, although its act was a valid answer with two options. Fleet kept the same item once the text was reworded as a noun phrase. The refusal does not say which rule applied.",
    steps: ["Call waiting_for with one item: text Approve the plan, act kind answer, two options.", "Read the refusal.", "Call it again with the text The plan. It is kept."],
    errors: ["fleet.waiting_item_refused: item plan reads as an action"],
    logs: [],
    at: ago(2),
    kept: false,
  },
  {
    id: "report-sketch-icon",
    session_id: SKETCH,
    job_title: "Draw the dispatch retry as a sketch",
    title: "show_sketch refused an icon the list allows",
    what: "A scene naming the icon git-branch was refused as unknown, though the icon list in the tool's description includes it. The same scene drew once the icon was replaced by git-merge.",
    steps: ["Call show_sketch with a node whose icon is git-branch.", "Read the refusal.", "Replace the icon with git-merge and call again."],
    errors: ["sketch.icon_unknown: git-branch is not in the icon list", "scene.nodes[2].icon"],
    logs: [".armada/scratch/scene-retry.json"],
    at: ago(26),
    kept: true,
  },
];

const SEED: IssuesSeed = [
  {
    tracker: { manifestId: "armada", state: "read", name: "GitHub", open: "is:open", assigned: "is:open assignee:@me", saved: [] },
    tickets: ARMADA_TICKETS,
    me: "nick",
    language: "terms",
  },
  {
    tracker: { manifestId: "storefront", state: "read", name: "GitHub", open: "is:open", assigned: "is:open assignee:@me", saved: [] },
    tickets: STOREFRONT_TICKETS,
    me: "nick",
    language: "terms",
  },
];

function build(): Scenario {
  const base = holding("reported-bugs", "Bugs agents reported in Armada: the Issues tab's Reported bugs band, its reader, and the mark on the reporting Jobs' rows in the Cockpit", [ARMADA, STOREFRONT]);
  return {
    ...base,
    state: { ...base.state, repository: null },
    draft: { sessions: (_control, more = []) => sessionsStore([...SESSIONS(), ...more], [], new Map(), []), issuesTab: SEED, reportedBugs: REPORTS() },
  };
}

export const reportedBugs: Scenario = build();
