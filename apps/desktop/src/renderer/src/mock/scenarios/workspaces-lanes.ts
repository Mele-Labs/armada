// The Workspace `my-store` and two Jobs in it: the Workflows page draws the `feature` workflow over the Ticket, PR and Deploy lanes, and a Job's Lanes view draws
// the same lanes with one for each ticket, pull request and deploy it holds, moment by moment as a walk's `later` lets time pass. Mock only: no Fleet serves a
// Workspace, a lifecycle or a rule yet. The walk `workspaces-lanes` plays it.

import type { RepositorySummary, WaitingItem } from "@armada/protocol";
import type { Session, SessionAttachment, SessionRow } from "@armada/screens/src/draft/sessions";
import { repository, workflow } from "@armada/screens/src/fixtures/repository";

import { connected } from "../moment";
import type { Scenario } from "../moment";
import type { PullRequestPage } from "@armada/protocol";

import { manifesting } from "../manifest-fake";
import { sessionsStore } from "../sessions/script";
import { sharedPanelReads } from "../shared-panel-reads";
import type { PanelReads, PullRequestRead } from "../../shared-panel/reads";
import { workflowsServed } from "../workflows-fleet";
import { API, ASK_DRAFT, COMMENTS, MY_STORE, PROPOSALS, STORE_DEFINITIONS, STORE_ENDS, WEB, WORKING, jobAt, jobB } from "../workspaces-lanes-data";

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
  attachments: [pull(815, "merged")],
  work: {
    proposal: { workflow: "feature", picked: ["workflow"] },
    workflow: { name: "feature", steps: STEPS, at: STEPS.length, visits: VISITS.slice() },
  },
  lanes: jobB(),
};

const ASK_ID = `draft:${ASK_DRAFT}`;
/** The card is the rule's own: it is named for the rule, and says where it goes and to whom. */
const ASK_TITLE = "Ask for review";
const ASK_RULE = { where: "Comment on PR #812", to: "Maya and Jun" } as const;
const ASK_TEXT = "Maya, Jun: PR #812 adds gift cards to the cart and the balance endpoint. Could you review it today?";

const reply = (id: string, at: string, text: string): SessionRow => ({ id, at, kind: "message", from: { kind: "agent" }, text });

/** The pull request's state at a moment: a draft when it opens, open once reviewers are on, merged when Land is done. */
const prAt = (moment: number) => (moment >= 6 ? "merged" : moment >= 3 ? "open" : "draft");

function sessions() {
  const store = sessionsStore([jobA, jobOther]);
  const mine = new Set([JOB, OTHER]);
  let moment = 0;
  const sent: string[] = [];
  let from: readonly Session[] | undefined;
  let only: readonly Session[] = [];
  /** The Job at a moment: its lanes, the step the rail is on, its pull request, and whether it is done. */
  const at = (n: number) => (one: Session): Session => {
    const lanes = jobAt(n, sent);
    const step = Math.max(0, Math.min(lanes.at, STEPS.length));
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
      work: { ...one.work, workflow: { name: "feature", steps: STEPS, at: step, visits: VISITS.slice(0, Math.min(step, STEPS.length - 1) + 1) } },
      // The deploy is healthy and the tickets are closed: nothing is left, and the row reads done.
      standing: n >= 7 ? "done" : "replied",
      ...(n >= 7 ? { done: { since: local("13:52") } } : {}),
    };
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
    draft: { sessions, workspace: { workspace: MY_STORE, proposals: PROPOSALS }, sharedPanel: panelReads },
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

export const workspacesLanes: Scenario = build();
