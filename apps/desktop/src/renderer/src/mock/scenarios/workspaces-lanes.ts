// The Workspace `my-store` and two Jobs in it: the Workflows page draws the `feature` workflow over the Ticket, PR and Deploy lanes, and a Job's Lanes view draws
// the same lanes with one for each ticket, pull request and deploy it holds, moment by moment as a walk's `later` lets time pass. Mock only: no Fleet serves a
// Workspace, a lifecycle or a rule yet. The walk `workspaces-lanes` plays it.

import type { RepositorySummary, WaitingItem } from "@armada/protocol";
import type { Session, SessionRow } from "@armada/screens/src/draft/sessions";
import { repository, workflow } from "@armada/screens/src/fixtures/repository";

import { connected } from "../moment";
import type { Scenario } from "../moment";
import { manifesting } from "../manifest-fake";
import { sessionsStore } from "../sessions/script";
import { workflowsServed } from "../workflows-fleet";
import { API, MY_STORE, PROPOSALS, STORE_DEFINITIONS, STORE_ENDS, WEB, jobAt, jobB } from "../workspaces-lanes-data";

const JOB = "gift-cards";
const OTHER = "cart-totals";

const WEB_ID = repository().manifest!.id;
const API_ID = "01M1CNPKTV0018H2M1CXDNBK07";

const STORE_WEB: RepositorySummary = {
  root: "/Users/user/store-web",
  records_root: "/Users/user/Library/Application Support/Armada/records/store-web",
  manifest: { ...repository().manifest!, id: WEB_ID, repository: WEB, path: "store-web/armada.yml" },
};
const STORE_API: RepositorySummary = {
  root: "/Users/user/store-api",
  records_root: "/Users/user/Library/Application Support/Armada/records/store-api",
  manifest: { ...repository().manifest!, id: API_ID, repository: API, path: "store-api/armada.yml" },
};

const STEPS = ["Plan", "Build", "Open PR", "Get it ready", "Answer reviews", "Land"];
const day = (time: string) => `2026-10-28T${time}:00Z`;

/** When each step began, up to the one the Job is at. */
const VISITS = [
  { step: 0, since: day("12:04") },
  { step: 1, since: day("12:12") },
  { step: 2, since: day("12:31") },
  { step: 3, since: day("12:40") },
  { step: 4, since: day("13:02") },
  { step: 5, since: day("13:20") },
] as const;

const brief: SessionRow = { id: "gc-1", at: "11:58:00", kind: "message", from: { kind: "you" }, text: "Add gift cards to the cart. Tickets ARM-41 and ARM-42." };

const jobA: Session = {
  id: JOB,
  title: "Add gift cards to the cart",
  manifestId: WEB_ID,
  turn: { state: "idle" },
  standing: "replied",
  lastTurnAt: new Date().toISOString(),
  rows: [brief],
  attachments: [],
  work: {
    proposal: { workflow: "feature", picked: ["workflow"] },
    workflow: { name: "feature", steps: STEPS, visits: [] },
  },
  lanes: jobAt(0),
};

const jobOther: Session = {
  id: OTHER,
  title: "Fix cart rounding",
  manifestId: WEB_ID,
  turn: { state: "idle" },
  standing: "replied",
  lastTurnAt: new Date().toISOString(),
  rows: [{ id: "ct-1", at: "09:02:00", kind: "message", from: { kind: "you" }, text: "The cart total is a cent off when a discount applies. ARM-52." }],
  attachments: [],
  work: {
    proposal: { workflow: "feature", picked: ["workflow"] },
    workflow: { name: "feature", steps: STEPS, at: 5, visits: VISITS.slice() },
  },
  lanes: jobB(),
};

const ASK_ID = "draft:ra-1";
const ASK_TITLE = "Review ask for PR #812";
const ASK_TEXT = "Maya, Jun: PR #812 adds gift cards to the cart and the balance endpoint. Could you review it today?";

const reply = (id: string, text: string): SessionRow => ({ id, at: new Date().toISOString(), kind: "message", from: { kind: "agent" }, text });

function sessions() {
  const store = sessionsStore([jobA, jobOther]);
  const mine = new Set([JOB, OTHER]);
  let moment = 0;
  const sent: string[] = [];
  let from: readonly Session[] | undefined;
  let only: readonly Session[] = [];
  /** The Job at a moment: its lanes, and the step the rail is on. */
  const at = (n: number) => (one: Session): Session => {
    const lanes = jobAt(n, sent);
    return {
      ...one,
      lanes,
      work: { ...one.work, workflow: { name: "feature", steps: STEPS, at: Math.max(0, Math.min(lanes.at, STEPS.length - 1)), visits: VISITS.slice(0, Math.max(0, Math.min(lanes.at, STEPS.length - 1)) + 1) } },
    };
  };
  return {
    ...store,
    get: () => {
      const all = store.get();
      if (all !== from) {
        from = all;
        only = all.filter((one) => mine.has(one.id));
      }
      return only;
    },
    later() {
      moment = Math.min(moment + 1, 7);
      store.edit(JOB, (one) => {
        const next = at(moment)(one);
        // The review ask is drafted for his press: a card in the thread and a Needs you item.
        if (moment === 3) {
          const item: WaitingItem = { id: ASK_ID, text: ASK_TITLE, since: new Date().toISOString(), source: "draft", act: { kind: "answer", target: ASK_ID }, options: [{ label: "Approve" }] };
          return { ...next, rows: [...next.rows, { id: "ra-1", at: new Date().toISOString(), kind: "draft", title: ASK_TITLE, text: ASK_TEXT }], waitingFor: [item] };
        }
        return next;
      });
    },
    // The ask is sent: the card settles and the item goes.
    answerWaiting: (id: string, itemId: string, given: { choice?: number; text?: string; fixed?: string }) => {
      if (itemId !== ASK_ID) return store.answerWaiting?.(id, itemId, given);
      store.edit(id, (one) => ({
        ...one,
        rows: one.rows.map((row) => (row.id === "ra-1" && row.kind === "draft" ? { ...row, settled: { how: "approved" as const, at: new Date().toISOString() } } : row)),
        waitingFor: (one.waitingFor ?? []).filter((it) => it.id !== ASK_ID),
      }));
    },
    // A comment sent to the Job: it reads as sent on the lane.
    sendComment: (id: string, comment: string) => {
      if (!sent.includes(comment)) sent.push(comment);
      store.edit(id, at(moment));
    },
    // The person tells this Job to skip the deploy watch: the Deploy lane marks the change.
    send: (id: string, message: Parameters<typeof store.send>[1]) => {
      store.send(id, message);
      if (id !== JOB || !/skip the deploy watch/i.test(message.text)) return;
      moment = 8;
      window.setTimeout(
        () =>
          store.edit(JOB, (one) => ({
            ...at(8)(one),
            turn: { state: "idle" },
            standing: "replied",
            rows: [...one.rows, reply("gc-skip", "Skipping the deploy watch on this Job.")],
          })),
        900,
      );
    },
  };
}

function build(): Scenario {
  const base: Scenario = { name: "workspaces-lanes", says: "The Workspace my-store: the feature workflow over its Ticket, PR and Deploy lanes, and a Job drawing the same lanes as its tickets, pull request and deploy move", state: connected([workflow()], [STORE_WEB, STORE_API]) };
  return {
    ...base,
    state: { ...base.state, repository: STORE_WEB.root, health: { state: "read", health: { probes: [], not_probed: [], helm_action_authority: "acting", workflows_left_out: false } } },
    // Workflows over my-store's own definitions; the commands a workflow names are the ones the Manifest declares.
    behaves: (fleet) => ({
      ...workflowsServed(() => fleet.state().holds.repositories?.find((one) => one.root === fleet.state().repository)?.manifest?.id ?? null, { definitions: STORE_DEFINITIONS, ends: STORE_ENDS, nudges: [] }),
      readManifestFile: manifesting().behaves?.(fleet).readManifestFile!,
    }),
    draft: { sessions, workspace: { workspace: MY_STORE, proposals: PROPOSALS } },
  };
}

export const workspacesLanes: Scenario = build();
