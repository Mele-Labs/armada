// The Issues tab on All, four repositories served: storefront keeps its issues on GitHub (labels, milestones, a project with a
// Status), ops in Jira (keys like ABC-41 and a dozen fields), billing in Shortcut, read through a tracker mod he was taught,
// and mobile names Linear, which nothing reads. Four Jobs work issues: one running, one waiting on him, one done, one running in
// billing. The model's answer to Describe a query is canned per repository. The walk `issues-tab` plays it.

import type { RepositorySummary } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";
import type { Session } from "@armada/screens/src/draft/sessions";

import type { TicketField, TicketRead } from "../../issues-tab/model";
import { holding } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";
import type { IssuesSeed } from "../slices/issues";

const served = (id: string): RepositorySummary => ({
  root: `/Users/user/code/${id}`,
  records_root: `/records/${id}`,
  manifest: { ...repository().manifest!, id, repository: id, path: `${id}/armada.yml` },
});
const STOREFRONT = served("storefront");
const OPS = served("ops");
const BILLING = served("billing");
const MOBILE = served("mobile");

const ago = (hours: number) => new Date(Date.now() - hours * 3_600_000).toISOString();
const f = (name: string, ...values: string[]): TicketField => ({ name, values });

type Draft = Omit<TicketRead, "updated_at" | "linked_pull_request" | "linked_pull_requests" | "excerpt" | "holders"> & { hours: number; linked?: TicketRead["linked_pull_requests"] };
const ticket = ({ hours, linked = [], ...one }: Draft): TicketRead => ({ ...one, updated_at: ago(hours), excerpt: one.body.split("\n")[0] ?? "", linked_pull_request: linked.length > 0, linked_pull_requests: linked, holders: [] });

const GH = "https://github.com/acme/storefront/issues/";
const STOREFRONT_TICKETS: TicketRead[] = [
  ticket({
    reference: "#2810",
    title: "Coupon field accepts an expired code",
    url: `${GH}2810`,
    state: "open",
    author: "dana",
    assignees: ["nick"],
    hours: 2,
    fields: [f("Labels", "bug", "checkout"), f("Milestone", "v0.9"), f("Projects", "Storefront roadmap: In progress")],
    body: "A code past its end date is still taken at checkout, and the discount applies.\n\n**Steps**\n\n1. Add any item to the cart.\n2. Enter `SPRING24` in the coupon field.\n3. The total drops by 20%.\n\nThe expiry check reads the server's date in UTC and compares it with a local date.",
    linked: [
      { number: 2814, title: "Compare coupon expiry in UTC", address: "https://github.com/acme/storefront/pull/2814", state: "open", checks: { state: "running", said: "Checks running" } },
      { number: 2809, title: "Read coupon dates as local time", address: "https://github.com/acme/storefront/pull/2809", state: "closed" },
    ],
  }),
  ticket({
    reference: "#2803",
    title: "Order history pages past the last order",
    url: `${GH}2803`,
    state: "open",
    author: "sam",
    assignees: [],
    hours: 5,
    fields: [f("Labels", "bug"), f("Milestone", "v0.9"), f("Projects", "Storefront roadmap: Todo")],
    body: "Next stays enabled on the last page of order history and loads an empty page.",
  }),
  ticket({
    reference: "#2799",
    title: "Checkout button stays disabled after a declined card",
    url: `${GH}2799`,
    state: "open",
    author: "nick",
    assignees: [],
    hours: 9,
    fields: [f("Labels", "bug", "checkout"), f("Milestone", "v0.9"), f("Projects", "Storefront roadmap: Todo")],
    body: "After the payment provider declines a card, the Pay button stays disabled until the page reloads.",
  }),
  ticket({
    reference: "#2796",
    title: "Round the cart totals to the currency's minor unit",
    url: `${GH}2796`,
    state: "open",
    author: "dana",
    assignees: ["nick"],
    hours: 20,
    fields: [f("Labels", "enhancement"), f("Milestone", "v0.9"), f("Projects", "Storefront roadmap: In review")],
    body: "Totals in JPY show two decimals. Round each total to the currency's minor unit before it is drawn.",
    linked: [{ number: 2801, title: "Round totals by currency", address: "https://github.com/acme/storefront/pull/2801", state: "merged", checks: { state: "passed", said: "Checks passed" } }],
  }),
  ticket({
    reference: "#2788",
    title: "Search returns archived products",
    url: `${GH}2788`,
    state: "open",
    author: "lee",
    assignees: ["lee"],
    hours: 30,
    fields: [f("Labels", "bug", "search"), f("Milestone", "v1.0"), f("Projects", "Storefront roadmap: Todo")],
    body: "Archived products appear in search results and lead to a page that says the product is gone.",
  }),
  ticket({
    reference: "#2771",
    title: "Lay out the invoice PDF on A4 and Letter",
    url: `${GH}2771`,
    state: "open",
    author: "sam",
    assignees: [],
    hours: 52,
    fields: [f("Labels", "enhancement", "invoices"), f("Projects", "Storefront roadmap: Todo")],
    body: "The invoice is laid out for Letter only, and its footer is cut off on A4.",
  }),
];

const JIRA = "https://acme.atlassian.net/browse/";
const OPS_TICKETS: TicketRead[] = [
  ticket({
    reference: "ABC-41",
    title: "Rotate the staging TLS certificates before they lapse",
    url: `${JIRA}ABC-41`,
    state: "open",
    author: "Priya Shah",
    assignees: ["Nick Mele"],
    hours: 3,
    fields: [
      f("Status", "In Progress"),
      f("Priority", "Highest"),
      f("Issue type", "Task"),
      f("Sprint", "Ops 2026.21"),
      f("Components", "Ingress", "Secrets"),
      f("Fix versions", "2026.11"),
      f("Story points", "3"),
      f("Labels", "security"),
      f("Epic link", "ABC-12 Staging hardening"),
      f("Due date", "2026-10-29"),
    ],
    body: "The staging certificates lapse on 2 Nov. Rotate them through the secrets store and restart the ingress pods one at a time.\n\nThe production rotation is ABC-44.",
    linked: [{ number: 418, title: "Rotate the staging ingress certificates", address: "https://github.com/acme/ops/pull/418", state: "open", checks: { state: "failed", said: "Checks failed: ingress-smoke" } }],
  }),
  ticket({
    reference: "ABC-38",
    title: "Alert when the backup job misses its window",
    url: `${JIRA}ABC-38`,
    state: "open",
    author: "Tom Reyes",
    assignees: [],
    hours: 7,
    fields: [f("Status", "To Do"), f("Priority", "High"), f("Issue type", "Story"), f("Sprint", "Ops 2026.21"), f("Components", "Backups"), f("Fix versions", "2026.11"), f("Story points", "5"), f("Epic link", "ABC-9 Backups you can trust")],
    body: "The nightly backup failed twice last month and nobody heard. Page the on-call when it has not finished by 04:00 UTC.",
  }),
  ticket({
    reference: "ABC-36",
    title: "Terraform plan shows drift on the DNS zone",
    url: `${JIRA}ABC-36`,
    state: "open",
    author: "Nick Mele",
    assignees: ["Nick Mele"],
    hours: 26,
    fields: [f("Status", "To Do"), f("Priority", "Medium"), f("Issue type", "Bug"), f("Components", "DNS"), f("Story points", "2")],
    body: "Every plan wants to recreate two TXT records that someone edited by hand.",
  }),
  ticket({
    reference: "ABC-33",
    title: "Move the log shipper off the deprecated agent",
    url: `${JIRA}ABC-33`,
    state: "open",
    author: "Priya Shah",
    assignees: [],
    hours: 40,
    fields: [f("Status", "To Do"), f("Priority", "High"), f("Issue type", "Bug"), f("Sprint", "Ops 2026.21"), f("Components", "Logging"), f("Fix versions", "2026.11"), f("Story points", "8")],
    body: "The agent stops getting fixes in December.",
  }),
  ticket({
    reference: "OPS-7",
    title: "Document the on-call handover",
    url: `${JIRA}OPS-7`,
    state: "open",
    author: "Tom Reyes",
    assignees: ["Tom Reyes"],
    hours: 70,
    fields: [f("Status", "In Review"), f("Priority", "Low"), f("Issue type", "Task"), f("Components", "Runbooks")],
    body: "What the outgoing on-call writes down, and where.",
  }),
];

const SHORTCUT = "https://app.shortcut.com/acme/story/";
const BILLING_TICKETS: TicketRead[] = [
  ticket({
    reference: "sc-1832",
    title: "Prorate a plan change made mid cycle",
    url: `${SHORTCUT}1832`,
    state: "open",
    author: "ines",
    assignees: ["nick"],
    hours: 4,
    fields: [f("State", "In Development"), f("Type", "feature"), f("Iteration", "Billing 44"), f("Estimate", "5"), f("Labels", "invoices")],
    body: "An upgrade on day 10 of 30 charges the whole month at the new price. Charge the rest of the cycle at the difference.",
  }),
  ticket({
    reference: "sc-1827",
    title: "Webhook retries charge the card twice",
    url: `${SHORTCUT}1827`,
    state: "open",
    author: "ines",
    assignees: [],
    hours: 11,
    fields: [f("State", "Unstarted"), f("Type", "bug"), f("Iteration", "Billing 44"), f("Estimate", "3"), f("Labels", "payments")],
    body: "A retried `invoice.paid` webhook creates a second charge. Key the charge on the event id.",
  }),
  ticket({
    reference: "sc-1819",
    title: "Tax line missing on Canadian invoices",
    url: `${SHORTCUT}1819`,
    state: "open",
    author: "marc",
    assignees: [],
    hours: 33,
    fields: [f("State", "Unstarted"), f("Type", "bug"), f("Estimate", "2"), f("Labels", "invoices", "tax")],
    body: "GST and PST are summed into the total but not listed as lines.",
  }),
];

/** Jobs at work on issues: the Cockpit draws the same four. */
const now = () => new Date().toISOString();
const say = (id: string, text: string) => ({ id, at: now(), kind: "message", from: { kind: "agent" }, text }) as const;
const ask = (id: string, text: string) => ({ id, at: now(), kind: "message", from: { kind: "you" }, text }) as const;

const SESSIONS = (): Session[] => [
  {
    id: "01ISSUECOUPONAAAAAAAAAAAAA",
    title: "Fix the coupon expiry check",
    manifestId: "storefront",
    turn: { state: "working" },
    lastTurnAt: now(),
    standing: "working",
    rows: [ask("c1-u", "#2810 Coupon field accepts an expired code"), say("c1-a", "The expiry check compares a UTC date with a local one. Changing it to compare in UTC.")],
    attachments: [],
  },
  {
    id: "01ISSUECERTSBBBBBBBBBBBBBB",
    title: "Rotate the staging certificates",
    manifestId: "ops",
    turn: { state: "idle" },
    lastTurnAt: now(),
    standing: "idle",
    rows: [ask("r1-u", "ABC-41 Rotate the staging TLS certificates before they lapse"), say("r1-a", "The new certificates are in the secrets store. Restart the ingress pods now, or in tonight's window?")],
    attachments: [],
    asked: {
      command: "Restart the ingress pods now, or in tonight's window?",
      call: "r1-ask",
      offers: ["allow_once", "refuse"],
      questions: [
        {
          question: "Restart the ingress pods now, or in tonight's window?",
          header: "Restart",
          multi_select: false,
          options: [
            { label: "Now", description: "One pod at a time" },
            { label: "Tonight", description: "In the 22:00 window" },
          ],
        },
      ],
    },
  },
  {
    id: "01ISSUEROUNDCCCCCCCCCCCCCC",
    title: "Round the cart totals",
    manifestId: "storefront",
    turn: { state: "idle" },
    lastTurnAt: ago(6),
    standing: "done",
    done: { since: ago(6) },
    rows: [ask("t1-u", "#2796 Round the cart totals to the currency's minor unit"), say("t1-a", "Merged #2801. Totals round by currency.")],
    attachments: [],
    waitingFor: [{ id: `done:${ago(6)}`, text: "Done", since: ago(6), source: "done" }],
  },
  {
    id: "01ISSUEPRORATEDDDDDDDDDDDD",
    title: "Prorate mid cycle plan changes",
    manifestId: "billing",
    turn: { state: "working" },
    lastTurnAt: now(),
    standing: "working",
    rows: [ask("p1-u", "sc-1832 Prorate a plan change made mid cycle"), say("p1-a", "Reading how the invoice lines are built.")],
    attachments: [],
  },
];

const SEED: IssuesSeed = [
  {
    tracker: {
      manifestId: "storefront",
      state: "read",
      name: "GitHub",
      open: "is:open",
      assigned: "is:open assignee:@me",
      saved: [{ id: "checkout-bugs", name: "Checkout bugs", words: "Checkout bugs", query: "is:open label:bug label:checkout", row_fields: ["Labels", "Projects"] }],
    },
    held: { "#2810": ["01ISSUECOUPONAAAAAAAAAAAAA"], "#2796": ["01ISSUEROUNDCCCCCCCCCCCCCC"] },
    tickets: STOREFRONT_TICKETS,
    me: "nick",
    language: "terms",
    writes: { query: 'is:open label:bug milestone:"v0.9" no:assignee', row_fields: ["Labels", "Milestone"] },
  },
  {
    tracker: {
      manifestId: "ops",
      state: "read",
      name: "Jira",
      open: "statusCategory != Done ORDER BY updated DESC",
      assigned: "assignee = currentUser() AND statusCategory != Done",
      saved: [],
    },
    held: { "ABC-41": ["01ISSUECERTSBBBBBBBBBBBBBB"] },
    tickets: OPS_TICKETS,
    me: "Nick Mele",
    language: "jql",
    writes: { query: "statusCategory != Done AND priority in (High, Highest) AND sprint in openSprints() AND assignee is EMPTY", row_fields: ["Priority", "Sprint"] },
  },
  {
    tracker: {
      manifestId: "billing",
      state: "read",
      name: "Shortcut",
      mod: "shortcut-tracker",
      open: "is:open",
      assigned: "is:open owner:@me",
      saved: [],
    },
    held: { "sc-1832": ["01ISSUEPRORATEDDDDDDDDDDDD"] },
    tickets: BILLING_TICKETS,
    me: "nick",
    language: "terms",
    writes: { query: "is:open type:bug no:owner state:Unstarted", row_fields: ["Type", "Labels"] },
  },
  {
    tracker: { manifestId: "mobile", state: "refused", named: "linear", said: "no tracker is called `linear`; the ones this build has are github and jira" },
  },
];

function build(): Scenario {
  const base = holding("issues-tab", "The Issues tab on All: GitHub, Jira, a taught Shortcut mod and an unreadable Linear, four Jobs working issues", [STOREFRONT, OPS, BILLING, MOBILE]);
  return {
    ...base,
    state: { ...base.state, repository: null },
    draft: { sessions: (_control, more = []) => sessionsStore([...SESSIONS(), ...more], [], new Map(), []), issuesTab: SEED },
  };
}

export const s321IssuesTab: Scenario = build();
