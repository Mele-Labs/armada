// The Workspace `my-store` and three Jobs in it: the Workflows page draws the `feature` workflow over the Ticket, PR and Deploy lanes, and a Job's Lanes view draws
// the same lanes with one for each ticket, pull request and deploy it holds, moment by moment as a walk's `later` lets time pass. The third Job goes through app feedback:
// it changes its own copy of the workflow, starts two Jobs that each get a lane, one of them carrying the ticket it was started for, waits on both, and offers `dispatch` as a workflow. `later` moves the Job on show. The Workspace is the page's own mock
// (`workspaces-pieces` is the one a Fleet serves). The walks `workspaces-lanes`, `workspaces-lanes-notes` and `workspaces-lanes-acts` play it, and `ticket-on-job-lane` plays its second row.

import type { RepositorySummary, WaitingItem } from "@armada/protocol";
import type { JobCopyActs, Session, SessionAttachment, SessionRow } from "@armada/screens/src/draft/sessions";
import type { JobChange, SessionLanes } from "@armada/components";
import { repository, workflow } from "@armada/screens/src/fixtures/repository";

import { connected } from "../moment";
import type { Scenario } from "../moment";
import type { PullRequestPage } from "@armada/protocol";

import { manifesting } from "../manifest-fake";
import { sessionsStore } from "../sessions/script";
import { sharedPanelReads } from "../shared-panel-reads";
import type { PanelReads, PullRequestRead } from "../../shared-panel/reads";
import { offerWorkflow, workflowsServed } from "../workflows-fleet";
import { deckNow } from "../../cockpit/deck/store";
import { forceWideWindow } from "../../session-view/lanes-width";
import { API, ASK_DRAFT, COMMENTS, FEEDBACK, FEEDBACK_SKIPPED, FEEDBACK_STEPS, FEEDBACK_STEPS_CHANGED, FEEDBACK_WORKING, MY_STORE, PROPOSALS, STORE_DEFINITIONS, STORE_ENDS, WEB, WORKING, dispatchText, feedbackAt, jobAt, jobB } from "../workspaces-lanes-data";
import { throughWire } from "../workspaces-lanes-wire";

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

// A Job's lanes are drawn from what Fleet would serve of it (`workspaces-lanes-wire.ts`), as a real Job's are.
const lanesOfA = (moment: number, sent: readonly string[] = []) => throughWire(JOB, jobAt(moment, sent));
const lanesOfFeedback = (moment: number) => throughWire(FEEDBACK, feedbackAt(moment));

/** The mock's one clock: a time today, which the lanes' chips, the rail's steps and the thread's rows all read. The thread's own rows are stamped as `HH:MM:SS`. */
const local = (time: string) => {
  const [hours, minutes] = time.split(":").map(Number);
  return new Date(new Date().setHours(hours!, minutes!, 0, 0)).toISOString();
};

/** When each step began, up to the one the Job is at. */
const VISITS = [
  { step: 0, since: local("12:04") },
  { step: 1, since: local("12:12") },
  { step: 2, since: local("12:31") },
  { step: 3, since: local("12:40") },
  { step: 4, since: local("13:02") },
  { step: 5, since: local("13:20") },
] as const;

/** The pull request a Job opened, as the thread's ledger and the rail's route read it. */
const pull = (number: number, state: "draft" | "open" | "merged"): Extract<SessionAttachment, { kind: "pull_request" }> => ({
  kind: "pull_request",
  number,
  title: number === 812 ? "Add gift cards to the cart" : "Fix cart rounding",
  branch: number === 812 ? "feat/gift-cards" : "fix/cart-rounding",
  address: `https://git.example/store-web/pull/${number}`,
  checks: { state: "passed" },
  state,
  auto: false,
});

const brief: SessionRow = { id: "gc-1", at: "11:58:00", kind: "message", from: { kind: "you" }, text: "Add gift cards to the cart. Tickets ARM-41 and ARM-42." };
const SENT_AT = "14:04:10";
const SKIPPED_AT = "14:05:00";

const jobA: Session = {
  id: JOB,
  title: "Add gift cards to the cart",
  manifestId: WEB_ID,
  turn: { state: "working" },
  account: { for: brief.id, done: WORKING[0]!.done, now: WORKING[0]!.now, at: new Date().toISOString() },
  standing: "replied",
  lastTurnAt: new Date().toISOString(),
  rows: [brief],
  attachments: [],
  work: {
    proposal: { workflow: "feature", picked: ["workflow"] },
    workflow: { name: "feature", steps: STEPS, visits: [] },
  },
  lanes: lanesOfA(0),
};

const jobOther: Session = {
  id: OTHER,
  title: "Fix cart rounding",
  manifestId: WEB_ID,
  turn: { state: "idle" },
  standing: "replied",
  lastTurnAt: new Date().toISOString(),
  rows: [{ id: "ct-1", at: "09:02:00", kind: "message", from: { kind: "you" }, text: "The cart total is a cent off when a discount applies. ARM-52." }],
  attachments: [pull(815, "merged")],
  work: {
    proposal: { workflow: "feature", picked: ["workflow"] },
    workflow: { name: "feature", steps: STEPS, at: STEPS.length, visits: VISITS.slice() },
  },
  lanes: throughWire(OTHER, jobB()),
};

const feedbackBrief: SessionRow = { id: "fb-1", at: "14:02:00", kind: "message", from: { kind: "you" }, text: "Go through the app feedback and fix what is clear." };

const jobFeedback: Session = {
  id: FEEDBACK,
  title: "Go through app feedback",
  manifestId: WEB_ID,
  turn: { state: "working" },
  account: { for: feedbackBrief.id, done: FEEDBACK_WORKING[0]!.done, now: FEEDBACK_WORKING[0]!.now, at: new Date().toISOString() },
  standing: "replied",
  lastTurnAt: new Date().toISOString(),
  rows: [feedbackBrief],
  attachments: [],
  work: {
    proposal: { workflow: "feature", picked: ["workflow"] },
    workflow: { name: "feature", steps: FEEDBACK_STEPS, at: 1, visits: [{ step: 0, since: local("14:02") }, { step: 1, since: local("14:04") }] },
  },
  lanes: lanesOfFeedback(0),
};

/** What the Job says at each moment, and when it began the step it is at. The first moment has nothing to say yet. */
const FEEDBACK_SAYS: readonly { moment: number; row: SessionRow }[] = [
  { moment: 1, row: { id: "fb-2", at: "14:09:10", kind: "message", from: { kind: "agent" }, text: "Changed this Job's workflow: Walk and Refine skipped, Dispatch and Oversee added after Approve plan." } },
  { moment: 2, row: { id: "fb-3", at: "14:12:20", kind: "message", from: { kind: "agent" }, text: "Started two Jobs: Fix cart rounding, Tidy the PR panel landing." } },
  { moment: 3, row: { id: "fb-4", at: "14:18:00", kind: "message", from: { kind: "agent" }, text: "Fix cart rounding landed." } },
  { moment: 4, row: { id: "fb-5", at: "14:25:30", kind: "message", from: { kind: "agent" }, text: "Both Jobs landed." } },
  { moment: 4, row: { id: "fb-6", at: "14:25:40", kind: "message", from: { kind: "agent" }, text: "Proposed dispatch as a workflow: Assess, Plan, Dispatch, Oversee, Summarise. It waits on Approve in Workflows." } },
];
const FEEDBACK_VISITS = [
  { step: 0, since: local("14:02") },
  { step: 1, since: local("14:04") },
  { step: 2, since: local("14:07") },
  { step: 3, since: local("14:09") },
  { step: 4, since: local("14:12") },
  { step: 7, since: local("14:25") },
];

/** The feedback Job at a moment: its lanes, its workflow as the rail draws it and what it has said. */
const feedbackAtMoment = (n: number) => (one: Session): Session => {
  const lanes = withOwn(FEEDBACK, lanesOfFeedback(n));
  const doing = FEEDBACK_WORKING[n];
  const { account, ...rest } = one;
  void account;
  return {
    ...rest,
    turn: doing === undefined ? { state: "idle" } : { state: "working" },
    ...(doing === undefined ? {} : { account: { for: feedbackBrief.id, done: doing.done, now: doing.now, at: new Date().toISOString() } }),
    rows: [feedbackBrief, ...FEEDBACK_SAYS.filter((one) => one.moment <= n).map((one) => one.row)],
    lanes,
    work: {
      ...one.work,
      workflow: { name: "feature", steps: n >= 1 ? FEEDBACK_STEPS_CHANGED : FEEDBACK_STEPS, at: lanes.at, ...(n >= 1 ? { skipped: FEEDBACK_SKIPPED } : {}), visits: FEEDBACK_VISITS.filter((visit) => visit.step <= lanes.at && (n >= 1 || visit.step <= 1)) },
    },
  };
};

/** What the person added to a Job's own copy from its lanes: a mock's memory, laid over the fixture's lanes each time they are drawn. */
type Own = { steps: { id: string; label: string; after: string | undefined }[]; changes: JobChange[] };
const OWN = new Map<string, Own>();

/** The Job's lanes with what was added to its copy: each step after the one it names, the step the Job is at kept, and the change marked as made by the person. */
function withOwn(id: string, lanes: SessionLanes): SessionLanes {
  const mine = OWN.get(id);
  if (mine === undefined) return lanes;
  const steps = [...lanes.job.copy.steps];
  let { at } = lanes;
  let skipped = lanes.skipped;
  for (const one of mine.steps) {
    const index = one.after === undefined ? 0 : steps.findIndex((step) => step.id === one.after) + 1;
    steps.splice(index, 0, { id: one.id, label: one.label, col: index });
    if (index <= at) at += 1;
    skipped = skipped?.map((step) => (index <= step ? step + 1 : step));
  }
  return {
    ...lanes,
    at,
    ...(skipped === undefined ? {} : { skipped }),
    job: { ...lanes.job, copy: { ...lanes.job.copy, steps: steps.map((step, col) => ({ ...step, col })) }, changes: [...lanes.job.changes, ...mine.changes] },
  };
}

const ASK_ID = `draft:${ASK_DRAFT}`;
/** The card is the rule's own: it is named for the rule, and says where it goes and to whom. */
const ASK_TITLE = "Ask for review";
const ASK_RULE = { where: "Comment on PR #812", to: "Maya and Jun" } as const;
const ASK_TEXT = "Maya, Jun: PR #812 adds gift cards to the cart and the balance endpoint. Could you review it today?";

const reply = (id: string, at: string, text: string): SessionRow => ({ id, at, kind: "message", from: { kind: "agent" }, text });

/** The pull request's state at a moment: a draft when it opens, open once reviewers are on, merged when Land is done. */
const prAt = (moment: number) => (moment >= 6 ? "merged" : moment >= 3 ? "open" : "draft");

/** `begin` is the moment the feedback Job opens at, which `later` carries on from. */
function sessions(begin = 0) {
  const store = sessionsStore([jobA, jobOther, begin === 0 ? jobFeedback : feedbackAtMoment(begin)(jobFeedback)]);
  const mine = new Set([JOB, OTHER, FEEDBACK]);
  let moment = 0;
  let feedback = begin;
  const sent: string[] = [];
  let from: readonly Session[] | undefined;
  let only: readonly Session[] = [];
  /** The Job at a moment: its lanes, the step the rail is on, its pull request, and whether it is done. */
  const at = (n: number) => (one: Session): Session => {
    const lanes = withOwn(JOB, lanesOfA(n, sent));
    const labels = OWN.has(JOB) ? lanes.job.copy.steps.map((one) => one.label) : STEPS;
    const step = Math.max(0, Math.min(lanes.at, labels.length));
    const { done, standing, account, ...rest } = one;
    void done;
    void standing;
    void account;
    // The agent's own account of what it is on, while it works: the Now beat reads it.
    const doing = WORKING[n];
    return {
      ...rest,
      turn: doing === undefined ? { state: "idle" } : { state: "working" },
      ...(doing === undefined ? {} : { account: { for: brief.id, done: doing.done, now: doing.now, at: new Date().toISOString() } }),
      lanes,
      attachments: [...one.attachments.filter((it) => it.kind !== "pull_request"), ...(n >= 2 ? [pull(812, prAt(n))] : [])],
      work: { ...one.work, workflow: { name: "feature", steps: labels, at: step, visits: VISITS.slice(0, Math.min(step, labels.length - 1) + 1) } },
      // The deploy is healthy and the tickets are closed: nothing is left, and the row reads done.
      standing: n >= 7 ? "done" : "replied",
      ...(n >= 7 ? { done: { since: local("13:52") } } : {}),
    };
  };
  /** A Job drawn again at the moment it is on, after the person changed its copy. */
  const redrawn = (id: string): void => {
    if (id === JOB) store.edit(JOB, at(moment));
    else if (id === FEEDBACK) store.edit(FEEDBACK, feedbackAtMoment(feedback));
    else store.edit(id, (one) => ({ ...one, lanes: withOwn(id, throughWire(id, jobB())) }));
  };
  /** The review ask is sent: its card settles and the item goes. */
  const settled = (one: Session): Session => ({
    ...one,
    // The card is stamped with the time it was approved, as the chip's is.
    rows: one.rows.map((row) => (row.id === ASK_DRAFT && row.kind === "draft" && row.settled === undefined ? { ...row, at: "12:44:00", settled: { how: "approved" as const, at: "12:44:00" } } : row)),
    waitingFor: (one.waitingFor ?? []).filter((it) => it.id !== ASK_ID),
  });
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
      // Time passes for the Job on show.
      if (deckNow().sel?.key === `session:${FEEDBACK}`) {
        feedback = Math.min(feedback + 1, 4);
        // A walk cannot resize its window, so the scenario says it is wider than 1800px from here: Lanes then open in the spine.
        if (feedback === 3) forceWideWindow(true);
        // The Job offers `dispatch` as a workflow when both have landed, and the Workflows page lists it with a change waiting on Approve.
        if (feedback === 4) offerWorkflow(900, "dispatch", "kit", dispatchText());
        store.edit(FEEDBACK, feedbackAtMoment(feedback));
        return;
      }
      moment = Math.min(moment + 1, 7);
      store.edit(JOB, (one) => {
        const next = at(moment)(one);
        // The review ask is drafted for his press: a card in the thread and an item the rail carries.
        if (moment === 3) {
          const item: WaitingItem = { id: ASK_ID, text: ASK_TITLE, since: new Date().toISOString(), source: "draft", act: { kind: "answer", target: ASK_ID }, options: [{ label: "Send" }] };
          return { ...next, rows: [...next.rows, { id: ASK_DRAFT, at: "12:40:00", kind: "draft", title: ASK_TITLE, text: ASK_TEXT, rule: ASK_RULE }], waitingFor: [item] };
        }
        // Merged: the ask a person never sent has nothing left to wait for.
        if (moment >= 6 && !sent.includes("ask")) {
          sent.push("ask");
          return at(moment)(settled(next));
        }
        return next;
      });
    },
    answerWaiting: (id: string, itemId: string, given: { choice?: number; text?: string; fixed?: string }) => {
      if (itemId !== ASK_ID) return store.answerWaiting?.(id, itemId, given);
      if (!sent.includes("ask")) sent.push("ask");
      store.edit(id, (one) => at(moment)(settled(one)));
    },
    // A comment sent to the Job: it reads as sent on the lane.
    sendComment: (id: string, comment: string) => {
      if (!sent.includes(comment)) sent.push(comment);
      store.edit(id, at(moment));
    },
    // The person adds to a Job's own copy from its lanes: the lanes draw it at once and mark it as made by them. Keep for every Job, for the workflow or for a piece, is taken, and the mock holds no proposal for it.
    copy: {
      step: async (id, change) => {
        if (change.kind !== "add") return "The mock adds a step and nothing else";
        const mine = OWN.get(id) ?? { steps: [], changes: [] };
        const stepId = change.label.trim().toLowerCase().replace(/[^a-z0-9]+/g, "_");
        const after = change.after;
        const at = new Date().toISOString();
        const on = (store.get().find((one) => one.id === id)?.lanes?.job.copy.steps ?? []).find((one) => one.id === after)?.label;
        OWN.set(id, {
          steps: [...mine.steps, { id: stepId, label: change.label.trim(), after }],
          changes: [...mine.changes, { id: `workflow:${stepId}:${at}`, lane: "workflow", item: stepId, text: `Added ${on === undefined ? "first" : `after ${on}`} by you`, at }],
        });
        redrawn(id);
        return undefined;
      },
      piece: async (id, kind, piece, text) => {
        const declared = /^(?:rule_id|lifecycle_id|id):\s*(\S+)/m.exec(text ?? "")?.[1] ?? piece;
        if (text === undefined || declared === "") return "The file declares no id";
        // A rule is named as Fleet serves it once the Job's copy holds it: by the name its file gives.
        const name = kind === "rule" ? /^name:\s*(.+?)\s*$/m.exec(text)?.[1] : undefined;
        const mine = OWN.get(id) ?? { steps: [], changes: [] };
        const at = new Date().toISOString();
        OWN.set(id, {
          ...mine,
          changes: [...mine.changes, { id: `${kind}:${declared}:${at}`, lane: kind === "rule" ? "rule" : declared, item: kind === "rule" ? declared : "", text: "Added by you", at, ...(name === undefined ? {} : { name }) }],
        });
        redrawn(id);
        return undefined;
      },
      keep: async () => undefined,
      keepPiece: async () => undefined,
    } satisfies JobCopyActs,
    // The person tells this Job to skip the deploy watch: the Deploy lane marks the change. The message and the reply are stamped on the mock's clock.
    send: (id: string, message: Parameters<typeof store.send>[1]) => {
      const had = new Set((store.get().find((one) => one.id === id)?.rows ?? []).map((row) => row.id));
      store.send(id, message);
      if (id !== JOB) return;
      store.edit(JOB, (one) => ({ ...one, rows: one.rows.map((row) => (had.has(row.id) ? row : { ...row, at: SENT_AT })) }));
      if (!/skip the deploy watch/i.test(message.text)) return;
      moment = 8;
      window.setTimeout(
        () =>
          store.edit(JOB, (one) => ({
            ...at(8)(one),
            turn: { state: "idle" },
            rows: [...one.rows, reply("gc-skip", SKIPPED_AT, "Skipping the deploy watch on this Job.")],
          })),
        900,
      );
    },
  };
}

function build(name: string, says: string, begin = 0): Scenario {
  const base: Scenario = { name, says, state: connected([workflow()], [STORE_WEB, STORE_API]) };
  return {
    ...base,
    state: { ...base.state, repository: STORE_WEB.root, health: { state: "read", health: { probes: [], not_probed: [], helm_action_authority: "acting", workflows_left_out: false } } },
    // Workflows over my-store's own definitions; the commands a workflow names are the ones the Manifest declares.
    behaves: (fleet) => ({
      ...workflowsServed(() => fleet.state().holds.repositories?.find((one) => one.root === fleet.state().repository)?.manifest?.id ?? null, { definitions: STORE_DEFINITIONS, ends: STORE_ENDS, nudges: [] }),
      readManifestFile: manifesting().behaves?.(fleet).readManifestFile!,
    }),
    draft: { sessions: () => sessions(begin), workspace: { workspace: MY_STORE, proposals: PROPOSALS }, sharedPanel: panelReads },
  };
}

/**
 * The pull request panel reads PR #812 as the forge holds it: the three human comments, each as a conversation entry. The triage of each (what the agent made of it) is the Job's,
 * and the panel draws it beside the comment.
 */
const page = (): PullRequestPage => ({
  manifest_id: WEB_ID,
  number: 812,
  base: "main",
  author: "you",
  created_at: local("12:31"),
  additions: 142,
  deletions: 18,
  mergeable: "mergeable",
  review: "none",
  approvals: 0,
  body: "Adds gift cards to the cart and the balance endpoint.\n\nARM-41, ARM-42.",
  conversation: COMMENTS.map((one) => ({ kind: "comment" as const, author: one.author, at: local("13:02"), body: one.text })),
  checks: [{ name: "ci", state: "passed", seconds: 231, address: "https://git.example/store-web/pull/812/checks?check_run_id=1" }],
  files: [
    { path: "src/cart/totals.ts", status: "modified", additions: 40, deletions: 6 },
    { path: "api/orders.ts", status: "modified", additions: 96, deletions: 12 },
    { path: "README.md", status: "modified", additions: 6, deletions: 0 },
  ],
  commits: [{ sha: "b7e21f0", headline: "Add gift cards to the cart", author: "you", at: local("12:28") }],
});

const PATCHES: Record<string, string> = {
  "src/cart/totals.ts": "@@ -38,6 +38,9 @@\n   const subtotal = lines.reduce((sum, line) => sum + line.price * line.quantity, 0);\n-  return Math.round(subtotal) - discount;\n+  const credit = giftCards.reduce((sum, card) => sum + card.balance, 0);\n+  return Math.round(subtotal) - discount - credit;",
  "api/orders.ts": "@@ -115,4 +115,6 @@\n   const order = await create(cart);\n+  order.giftCardId = cart.giftCardId;\n+  await redeem(order.giftCardId, order.total);\n   return order;",
  "README.md": "@@ -7,3 +7,4 @@\n Gift cards are redeemed at checkout. \n+Balance is read from the card service.",
};

function panelReads(): PanelReads {
  const base = sharedPanelReads({ lines: [] });
  const read = (): PullRequestRead => ({ repository: "acme/store-web", root: STORE_WEB.root, facts: pull(812, "open"), page: page() });
  return {
    ...base,
    pullRequest: async (item) => (item.number === 812 ? read() : base.pullRequest?.(item)),
    pullRequestPatches: async (item) =>
      item.number === 812 ? { ok: true, files: Object.entries(PATCHES).map(([path, patch]) => ({ manifest_id: WEB_ID, number: 812, path, patch })) } : base.pullRequestPatches?.(item) ?? { ok: false as const, said: "The forge could not find this pull request." },
  };
}

export const workspacesLanes: Scenario = build(
  "workspaces-lanes",
  "The Workspace my-store: the feature and planning workflows over their lanes, edited by hand in the side panel, and a Job drawing the same lanes as its tickets, pull request and deploy move",
);

/** The feedback Job from the moment it has started two Jobs, one of them for a ticket: `ticket-on-job-lane`. */
export const workspacesLanesTicket: Scenario = build("ticket-on-job-lane", "The feedback Job with two Jobs started: one carries the ticket it was started for on its lane, and a ticket nobody started a Job for keeps a lane of its own", 2);
