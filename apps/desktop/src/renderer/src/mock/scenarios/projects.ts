// The Projects tab on All, two repositories served: armada keeps Pocket launch and Cockpit trust, site keeps Docs refresh.
// Pocket launch mixes tasks he wrote with issues he linked: one task a live Job holds, one held by a Job that finished and is
// still live (its Needs you item asks the same question as the row), one whose Job has ended and still asks on the row, a done
// task, a closed issue and an open one. The pairing task carries an issue and two subtasks. Both repositories' issues are read
// from their trackers, as the Issues tab reads them. Two takes of the select control and the card: `projects` (reticle) and
// `projects-plate`, each played by the walk of its name.

import type { RepositorySummary, WaitingItem } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";
import type { Session } from "@armada/screens/src/draft/sessions";

import type { TicketRead } from "../../issues-tab/model";
import type { Project } from "../../projects-tab/model";
import { holding } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";
import type { IssuesSeed } from "../slices/issues";
import { doneAskId, projectsLink, rowOfAsk } from "../slices/projects";

const served = (id: string): RepositorySummary => ({
  root: `/Users/user/code/${id}`,
  records_root: `/records/${id}`,
  manifest: { ...repository().manifest!, id, repository: id, path: `${id}/armada.yml` },
});
const ARMADA = served("armada");
const SITE = served("site");

const ago = (hours: number) => new Date(Date.now() - hours * 3_600_000).toISOString();
const now = () => new Date().toISOString();

const issue = (forge: string, number: number, title: string, body: string, state: "open" | "closed" = "open"): TicketRead => ({
  reference: `#${number}`,
  title,
  url: `https://github.com/acme/${forge}/issues/${number}`,
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
const SIGN_ROW = "task-sign";

const say = (id: string, text: string) => ({ id, at: now(), kind: "message", from: { kind: "agent" }, text }) as const;
const ask = (id: string, text: string) => ({ id, at: now(), kind: "message", from: { kind: "you" }, text }) as const;

/**
 * The done ask on the finished Job. Fleet derives no task item yet, so it rides as a Fleet answer item: two choices, answered
 * with `answer_waiting`, and nothing sent to the agent.
 */
const SIGN_ASK: WaitingItem = {
  id: doneAskId(SIGN_ROW),
  text: "Mark Sign the build done?",
  since: ago(1),
  source: "slot_choice",
  options: [{ label: "Mark done" }, { label: "Keep open" }],
  act: { kind: "answer", target: doneAskId(SIGN_ROW) },
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

const PROJECTS: Project[] = [
  {
    id: "pocket-launch",
    manifestId: "armada",
    name: "Pocket launch",
    usedAt: Date.now() - 3_600_000,
    rows: [
      {
        kind: "task",
        id: "task-pair",
        title: "Pair a phone by QR code",
        done: false,
        job: { id: PAIRING, title: "Wire the pairing screen" },
        issues: [{ reference: "#2815", title: "Pair code expires before the phone scans it", url: "https://github.com/acme/armada/issues/2815", state: "open" }],
        subtasks: [
          { kind: "task", id: "task-pair-draw", title: "Draw the code from the tunnel address", done: true },
          { kind: "task", id: "task-pair-scan", title: "Scan it on a small screen", done: false },
        ],
      },
      { kind: "issue", id: "issue-2801", reference: "#2801", title: "Pocket drops the socket when the Mac sleeps", url: "https://github.com/acme/armada/issues/2801", state: "open" },
      { kind: "task", id: SIGN_ROW, title: "Sign the build", done: false, job: { id: SIGNING, title: "Sign the release build" } },
      { kind: "task", id: "task-guide", title: "Write the install guide", done: false, job: { id: GUIDE, title: "Write the install guide" } },
      { kind: "task", id: "task-icon", title: "Pick the app icon", done: true },
      { kind: "issue", id: "issue-2790", reference: "#2790", title: "Tunnel address is not saved", url: "https://github.com/acme/armada/issues/2790", state: "closed" },
      { kind: "task", id: "task-small", title: "Test on a small phone", done: false },
    ],
  },
  {
    id: "cockpit-trust",
    manifestId: "armada",
    name: "Cockpit trust",
    usedAt: Date.now() - 7_200_000,
    rows: [
      { kind: "task", id: "task-replied", title: "Raise a call when a reply asks something", done: false },
      { kind: "task", id: "task-all-done", title: "Raise a call on all done", done: false },
    ],
  },
  {
    id: "docs-refresh",
    manifestId: "site",
    name: "Docs refresh",
    usedAt: Date.now() - 10_800_000,
    rows: [
      { kind: "issue", id: "issue-143", reference: "#143", title: "Search misses pages under guides", url: "https://github.com/acme/site/issues/143", state: "open" },
      { kind: "task", id: "task-pairing-page", title: "Write the pairing page", done: false },
    ],
  },
];

const TRACKERS: IssuesSeed = [
  { tracker: { manifestId: "armada", state: "read", name: "GitHub", open: "is:open", assigned: "is:open assignee:@me", saved: [] }, tickets: ARMADA_ISSUES, me: "nick", language: "terms" },
  { tracker: { manifestId: "site", state: "read", name: "GitHub", open: "is:open", assigned: "is:open assignee:@me", saved: [] }, tickets: SITE_ISSUES, me: "nick", language: "terms" },
];

function build(name: string, look: "reticle" | "plate"): Scenario {
  const base = holding(name, `Projects on All, the ${look} take: armada's Pocket launch and Cockpit trust, site's Docs refresh, with tasks held by a live, a finished and an ended Job`, [ARMADA, SITE]);
  return {
    ...base,
    state: { ...base.state, repository: null },
    draft: {
      // The Needs you item answers the row: Mark done (0) or Keep open (1). Anything else is the store's.
      sessions: (_control, more = []) => {
        const store = sessionsStore([...SESSIONS(), ...more], [], new Map(), []);
        projectsLink.jobs = store;
        return {
          ...store,
          answerWaiting: (id, itemId, given) => {
            const row = rowOfAsk(itemId);
            if (row === undefined) return store.answerWaiting?.(id, itemId, given);
            store.edit(id, (one) => ({ ...one, waitingFor: (one.waitingFor ?? []).filter((it) => it.id !== itemId) }));
            projectsLink.rows?.(row, given.choice === 0);
          },
        };
      },
      issuesTab: TRACKERS,
      projectsTab: PROJECTS,
      projectsLook: look,
    },
  };
}

export const s331Projects: Scenario = build("projects", "reticle");
export const s331ProjectsPlate: Scenario = build("projects-plate", "plate");
