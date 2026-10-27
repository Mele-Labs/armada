// The Projects tab on All, two repositories served. **A project spans repositories**: Pocket launch has tasks in armada and in
// site, Cockpit trust is armada's and Docs refresh is site's. Some tasks are in no project, in each repository, and two are done.
// Pocket launch's armada tasks mix tasks he wrote with issues he linked: one task a live Job holds, one held by a Job that finished and is
// still live (its Needs you item asks the same question as the row), one whose Job has ended and still asks on the row, a done
// task, a closed issue and an open one. The pairing task carries an issue and two subtasks.
// Both repositories' issues are read from their trackers, as the Issues tab reads them. Played by the walks `projects`,
// `projects-pass-3` and `projects-pass-4`.

import type { Projects, RepositorySummary, WaitingItem } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";
import type { Session } from "@armada/screens/src/draft/sessions";

import type { TicketRead } from "../../issues-tab/model";
import type { LinkedPull } from "../../projects-tab/model";
import { holding } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";
import type { IssuesSeed } from "../slices/issues";
import { doneAskId, projectOfAsk, projectsLink } from "../slices/projects";

const served = (id: string): RepositorySummary => ({
  root: `/Users/user/code/${id}`,
  records_root: `/records/${id}`,
  manifest: { ...repository().manifest!, id, repository: id, path: `${id}/armada.yml` },
});
const ARMADA = served("armada");
const SITE = served("site");

const ago = (hours: number) => new Date(Date.now() - hours * 3_600_000).toISOString();
const now = () => new Date().toISOString();

const FORGE = "https://forge.example/acme/";

const issue = (forge: string, number: number, title: string, body: string, state: "open" | "closed" = "open"): TicketRead => ({
  reference: `#${number}`,
  title,
  url: `${FORGE}${forge}/issues/${number}`,
  state,
  author: "nick",
  assignees: [],
  updated_at: ago(number % 7),
  excerpt: body,
  fields: [{ name: "Labels", values: ["pocket"] }],
  linked_pull_request: false,
  holders: [],
  body,
  linked_pull_requests: [],
});

const ARMADA_ISSUES: TicketRead[] = [
  issue("armada", 2819, "Push notices arrive twice on the phone", "Each notice shows twice when Pocket is open in two tabs."),
  issue("armada", 2815, "Pair code expires before the phone scans it", "The code lasts 30 seconds and the camera takes longer to focus."),
  issue("armada", 2812, "Pocket shows a blank screen after the tunnel drops", "The page stays blank until it is reloaded by hand."),
  issue("armada", 2801, "Pocket drops the socket when the Mac sleeps", "The socket closes on sleep and never reopens."),
  issue("armada", 2790, "Tunnel address is not saved", "The address is asked for again on every start.", "closed"),
];
const SITE_ISSUES: TicketRead[] = [
  issue("site", 143, "Search misses pages under guides", "A search for pairing finds nothing under /guides."),
  issue("site", 140, "Dark mode code blocks lose their contrast", "Comments in code blocks read at 2:1 on dark."),
];

/** The Jobs holding rows: one at work, one finished and still live, one ended. */
const PAIRING = "01PROJPAIRINGAAAAAAAAAAAAA";
const SIGNING = "01PROJSIGNINGBBBBBBBBBBBBB";
const GUIDE = "01PROJGUIDECCCCCCCCCCCCCCC";
/** A Job at work that holds no row yet: the walk links it to a task. */
const LISTING = "01PROJLISTINGDDDDDDDDDDDDD";
const SIGN_ROW = "task-sign";

const say = (id: string, text: string) => ({ id, at: now(), kind: "message", from: { kind: "agent" }, text }) as const;
const ask = (id: string, text: string) => ({ id, at: now(), kind: "message", from: { kind: "you" }, text }) as const;

/** The done ask on the finished Job: Fleet's `task_done` item for the project, two choices answered with `answer_waiting`. */
const SIGN_ASK: WaitingItem = {
  id: doneAskId("pocket-launch"),
  text: "Mark Sign the build done?",
  since: ago(1),
  source: "task_done",
  options: [{ label: "Mark done" }, { label: "Keep open" }],
  act: { kind: "answer", target: doneAskId("pocket-launch") },
};

const SESSIONS = (): Session[] => [
  {
    id: PAIRING,
    title: "Wire the pairing screen",
    manifestId: "armada",
    turn: { state: "working" },
    lastTurnAt: now(),
    standing: "working",
    rows: [ask("p1-u", "Pair a phone by QR code"), say("p1-a", "Drawing the code from the tunnel address.")],
    attachments: [],
  },
  {
    id: SIGNING,
    title: "Sign the release build",
    manifestId: "armada",
    turn: { state: "idle" },
    lastTurnAt: ago(1),
    standing: "done",
    done: { since: ago(1) },
    rows: [ask("s1-u", "Sign the build"), say("s1-a", "The release build is signed and notarised.")],
    attachments: [],
    waitingFor: [SIGN_ASK],
  },
  {
    id: LISTING,
    title: "Write the App Store listing",
    manifestId: "armada",
    turn: { state: "working" },
    lastTurnAt: now(),
    standing: "working",
    rows: [ask("l1-u", "Write the App Store listing"), say("l1-a", "Drafting the subtitle and the keywords.")],
    attachments: [],
  },
  {
    id: GUIDE,
    title: "Write the install guide",
    manifestId: "armada",
    turn: { state: "idle" },
    lastTurnAt: ago(20),
    standing: "done",
    dead: "ended",
    rows: [ask("g1-u", "Write the install guide"), say("g1-a", "The guide is in docs/pocket/install.md.")],
    attachments: [],
  },
];

/** armada's pull requests, which the link search offers. */
const PULLS: LinkedPull[] = [
  { number: 2874, title: "Fit the pairing screen to small phones", url: "https://forge.example/acme/armada/pull/2874", state: "open" },
  { number: 2870, title: "Sign the macOS build in CI", url: "https://forge.example/acme/armada/pull/2870", state: "merged" },
];

/** What Fleet lists: tasks and issue rows flat, a parent followed by its subtasks, a reference or a number with what the tracker or forge said of it. Each row names its repository. */
const PROJECTS: Projects = {
  projects: [
    {
      id: "pocket-launch",
      name: "Pocket launch",
      rows: [
        {
          kind: "task",
          id: "task-pair",
          manifest_id: "armada",
          title: "Pair a phone by QR code",
          notes: "Show the code as a QR code on the Mac. A phone on the same network scans it.",
          state: "held",
          held_by: { session_id: PAIRING, title: "Wire the pairing screen" },
          issues: [{ issue: "2815", title: "Pair code expires before the phone scans it", state: "open" }],
        },
        { kind: "task", id: "task-pair-draw", manifest_id: "armada", title: "Draw the code from the tunnel address", notes: "", state: "done", parent: "task-pair" },
        { kind: "task", id: "task-pair-scan", manifest_id: "armada", title: "Scan it on a small screen", notes: "", state: "open", parent: "task-pair" },
        { kind: "issue", id: "issue-2801", manifest_id: "armada", issue: "2801", title: "Pocket drops the socket when the Mac sleeps", state: "open" },
        { kind: "task", id: SIGN_ROW, manifest_id: "armada", title: "Sign the build", notes: "", state: "asked", held_by: { session_id: SIGNING, title: "Sign the release build" } },
        { kind: "task", id: "task-guide", manifest_id: "armada", title: "Write the install guide", notes: "", state: "asked", held_by: { session_id: GUIDE, title: "Write the install guide" } },
        { kind: "task", id: "task-icon", manifest_id: "armada", title: "Pick the app icon", notes: "", state: "done" },
        { kind: "issue", id: "issue-2790", manifest_id: "armada", issue: "2790", title: "Tunnel address is not saved", state: "closed" },
        { kind: "task", id: "task-small", manifest_id: "armada", title: "Test on a small phone", notes: "", state: "open", pulls: [{ number: 2874, title: PULLS[0]!.title, state: "open" }] },
        { kind: "task", id: "task-site-badge", manifest_id: "site", title: "Add the App Store badge to the home page", notes: "", state: "open" },
        { kind: "task", id: "task-pairing-page", manifest_id: "site", title: "Write the pairing page", notes: "", state: "open" },
      ],
    },
      {
        id: "cockpit-trust",
        name: "Cockpit trust",
        rows: [
        { kind: "task", id: "task-replied", manifest_id: "armada", title: "Raise a call when a reply asks something", notes: "", state: "open" },
        { kind: "task", id: "task-all-done", manifest_id: "armada", title: "Raise a call on all done", notes: "", state: "open" },
      ],
    },
      {
        id: "docs-refresh",
        name: "Docs refresh",
        rows: [
        { kind: "issue", id: "issue-143", manifest_id: "site", issue: "143", title: "Search misses pages under guides", state: "open" },
        { kind: "task", id: "task-docs-nav", manifest_id: "site", title: "Reorder the docs navigation", notes: "", state: "open" },
      ],
    },
  ],
  loose: [
    { kind: "task", id: "task-own-changelog", manifest_id: "armada", title: "Update the changelog", notes: "", state: "open" },
    { kind: "task", id: "task-own-key", manifest_id: "armada", title: "Rotate the signing key", notes: "", state: "done" },
    { kind: "task", id: "task-own-sitemap", manifest_id: "site", title: "Fix the sitemap dates", notes: "", state: "open" },
    { kind: "task", id: "task-own-404", manifest_id: "site", title: "Check the 404 page", notes: "", state: "open" },
    { kind: "task", id: "task-own-footer", manifest_id: "site", title: "Fix the footer links", notes: "", state: "done" },
  ],
};

const TRACKERS: IssuesSeed = [
  { tracker: { manifestId: "armada", state: "read", name: "GitHub", open: "is:open", assigned: "is:open assignee:@me", saved: [] }, tickets: ARMADA_ISSUES, me: "nick", language: "terms" },
  { tracker: { manifestId: "site", state: "read", name: "GitHub", open: "is:open", assigned: "is:open assignee:@me", saved: [] }, tickets: SITE_ISSUES, me: "nick", language: "terms" },
];

function build(name: string): Scenario {
  const base = holding(name, `Projects on All: Pocket launch across armada and site, armada's Cockpit trust, site's Docs refresh, tasks in no project, with tasks held by a live, a finished and an ended Job`, [ARMADA, SITE]);
  return {
    ...base,
    state: { ...base.state, repository: null },
    draft: {
      // The Needs you item answers the rows: Mark done (0) or Keep open (1), and Fleet's copy of them moves with it. Anything else is the store's.
      sessions: (_control, more = []) => {
        const store = sessionsStore([...SESSIONS(), ...more], [], new Map(), []);
        projectsLink.jobs = store;
        return {
          ...store,
          // Fleet holds the written tasks a Job starts with, so the rows show it from the moment the sheet starts one.
          start: (tag, root, request, tuning, after, held, projectRows) => {
            const id = store.start(tag, root, request, tuning, after, held);
            if (typeof id === "string" && projectRows !== undefined) projectsLink.hold?.(id, (request ?? "").trim().split(/\s+/).slice(0, 5).join(" "), projectRows);
            return id;
          },
          answerWaiting: (id, itemId, given) => {
            const project = projectOfAsk(itemId);
            if (project === undefined) return store.answerWaiting?.(id, itemId, given);
            store.edit(id, (one) => ({ ...one, waitingFor: (one.waitingFor ?? []).filter((it) => it.id !== itemId) }));
            projectsLink.settle?.(id, project, given.choice === 0);
          },
        };
      },
      issuesTab: TRACKERS,
      projectsTab: PROJECTS,
      projectsPulls: { armada: PULLS },
    },
  };
}

export const s331Projects: Scenario = build("projects");
