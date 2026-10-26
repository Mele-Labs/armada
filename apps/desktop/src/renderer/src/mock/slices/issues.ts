// Issues' member: four open issues for whichever repository asks, so the composer's list has
// something to draw. What Fleet reads and holds is `fleet::tracking`'s.

import type { IssueList, IssueRead, MergeLines, PullRequestState } from "@armada/protocol";

import type { IssuesApi } from "../../../../shared/api/issues";
import type { IssuesSource, QueryToSave, Read, SavedQuery, Ticket, TicketRead, Tracker, WrittenQuery } from "../../issues-tab/model";
import type { Slice } from "../fake-context";
import { unanswered } from "../moment";
import { REPOSITORY_ISSUES, finishedPullRequest, issueOfNumber } from "../numbered";
import { ISSUES } from "../shared-panel-reads";

/** A forge's name for an issue: its number after a `#`. */
const forge = (number: number): string => `#${number}`;

function listed(manifestId: string): IssueList {
  const own = REPOSITORY_ISSUES[manifestId];
  if (own !== undefined) return { manifest_id: manifestId, fetched_at: "2026-10-11T12:00:00Z", issues: own };
  return {
    manifest_id: manifestId,
    fetched_at: "2026-10-11T12:00:00Z",
    issues: [
      {
        reference: forge(412),
        title: "The log reader drops the last line",
        url: "https://forge.invalid/storefront/issues/412",
        labels: ["bug"],
        assignees: [],
        updated_at: "2026-10-11T11:40:00Z",
        excerpt: "A file that ends without a newline loses its last line in the viewer.",
        linked_pull_request: false,
        state: "open",
        fields: [],
        holders: [],
      },
      {
        reference: forge(398),
        title: "Checkout button stays disabled after a failed card",
        url: "https://forge.invalid/storefront/issues/398",
        labels: [],
        assignees: ["nick"],
        updated_at: "2026-10-10T09:12:00Z",
        excerpt: "",
        linked_pull_request: true,
        state: "open",
        fields: [],
        holders: [],
      },
      {
        reference: forge(405),
        title: "Coupon field accepts an expired code",
        url: "https://forge.invalid/storefront/issues/405",
        labels: ["bug"],
        assignees: [],
        updated_at: "2026-10-10T16:20:00Z",
        excerpt: "",
        linked_pull_request: false,
        state: "open",
        fields: [],
        holders: [],
      },
      {
        reference: forge(409),
        title: "Order history pages past the last order",
        url: "https://forge.invalid/storefront/issues/409",
        labels: [],
        assignees: [],
        updated_at: "2026-10-09T08:05:00Z",
        excerpt: "",
        linked_pull_request: false,
        state: "open",
        fields: [],
        holders: [],
      },
    ],
  };
}

/** An open pull request the merge lines hold, as `get_pull_request` states it. */
function openOn(manifestId: string, number: number, lines: MergeLines | null): PullRequestState | undefined {
  const one = (lines?.lines ?? []).flatMap((line) => line.hub?.pull_requests ?? []).find((pr) => pr.number === number);
  if (one === undefined) return undefined;
  const checks: PullRequestState["checks"] = one.ci === "failed" ? { state: "failed", failing: [] } : one.ci === "passed" ? { state: "passed" } : { state: "pending" };
  return { manifest_id: manifestId, number, state: "open", auto_merge: false, ...(one.queue === undefined ? {} : { queued: true }), checks, title: one.title, branch: one.branch, address: one.url };
}

function issueRead(manifestId: string, number: number): IssueRead | undefined {
  const one = issueOfNumber(manifestId, number);
  if (one === undefined) return undefined;
  return { reference: one.reference, title: one.title, url: one.url, state: "open", body: one.excerpt, author: "", opened_at: one.updated_at, labels: one.labels, assignees: one.assignees, comments: [], linked_pull_requests: [], fields: [], holders: [] };
}

/** An issue the side panel's fixtures hold, a tracker's key among them, as Fleet would answer it. */
function panelIssue(reference: string): IssueRead | undefined {
  const one = ISSUES[reference];
  if (one === undefined) return undefined;
  return {
    reference: one.number,
    title: one.title,
    url: one.address,
    state: one.state,
    body: one.body,
    author: one.author,
    opened_at: one.openedAt,
    labels: [...one.labels],
    assignees: [...one.assignees],
    comments: [...one.comments],
    linked_pull_requests: one.linked.map((pr) => ({ number: pr.number, title: pr.title, url: pr.address, state: pr.state })),
    fields: [],
    holders: [],
  };
}

export const issues: Slice<IssuesApi, Record<string, never>> = {
  name: "issues",
  state: {},
  api: (_scenario, fleet) => ({
    listIssues: async (manifestId) => ({ ok: true, value: listed(manifestId) }),
    // `#` reads these through `mock/numbered.ts` here; the side panel reads the same fixtures, as a Fleet's answers.
    getIssue: async (manifestId, reference) => {
      const one = issueRead(manifestId, Number(reference.replace(/^#/, ""))) ?? panelIssue(reference.replace(/^#/, ""));
      return one === undefined ? { ok: false, outcome: unanswered("/issues") } : { ok: true, value: one };
    },
    // The tab's tracker is the scenario's own (`issuesTabSource`), so the routes under it have nothing here to answer.
    searchIssues: async () => ({ ok: false, outcome: unanswered("/issue_queries/search") }),
    listIssueQueries: async () => ({ ok: false, outcome: unanswered("/issue_queries") }),
    writeIssueQuery: async () => ({ ok: false, outcome: unanswered("/issue_queries/write") }),
    saveIssueQuery: async () => ({ ok: false, outcome: unanswered("/issue_queries") }),
    forgetIssueQuery: async () => ({ ok: false, outcome: unanswered("/issue_queries/forget") }),
    getPullRequest: async (manifestId, number) => {
      const one = openOn(manifestId, number, fleet.state().mergeLines) ?? finishedPullRequest(manifestId, number);
      return one === undefined ? { ok: false, outcome: unanswered("/pull_requests") } : { ok: true, value: one };
    },
    // A forge that has not moved since the line was read: the refresh answers what a read would, and nothing changes.
    refreshPullRequest: async (manifestId, number) => {
      const one = openOn(manifestId, number, fleet.state().mergeLines) ?? finishedPullRequest(manifestId, number);
      return one === undefined ? { ok: false, outcome: unanswered("/pull_requests/refresh") } : { ok: true, value: one };
    },
  }),
};

// ---- The Issues tab's tracker, mock only ---------------------------------------------------------------------------------
// A walk has no Fleet to serve the tab. A scenario hands each repository's tracker, its issues read whole, who "me" is on it, the
// language its queries are in, and what the model writes for any words (`MockDraft.issuesTab`). Queries are run here, crudely:
// `key:value` terms for GitHub and a tracker mod, `AND`ed clauses for Jira's JQL, each matched against the issue's named
// fields. Enough for an edited query to match what it says.

/** One repository's tracker as the mock holds it. */
export type IssuesSeedRepository = {
  tracker: Tracker;
  tickets?: readonly TicketRead[];
  /** The Jobs working each issue, by reference, as Fleet's ledger holds them. */
  held?: Record<string, string[]>;
  me?: string;
  language?: "terms" | "jql";
  /** What the model writes, whatever the words. */
  writes?: WrittenQuery;
};

export type IssuesSeed = readonly IssuesSeedRepository[];

const plain = (text: string) => text.toLowerCase().replace(/[\s_-]/g, "").replace(/s$/, "");
const valuesOf = (ticket: Ticket, name: string): string[] => ticket.fields.find((one) => plain(one.name) === plain(name))?.values ?? [];
const has = (values: readonly string[], wanted: string) => values.some((one) => one.toLowerCase() === wanted.toLowerCase());
const unquote = (text: string) => text.trim().replace(/^"(.*)"$/, "$1");

/** GitHub's and a mod's `key:value` terms, quoted values whole. A bare word is looked for in the title. */
function byTerms(query: string, me: string): (ticket: Ticket) => boolean {
  const terms = query.match(/-?[\w-]+:"[^"]*"|-?[\w-]+:\S+|\S+/g) ?? [];
  return (ticket) =>
    terms.every((term) => {
      const negated = term.startsWith("-");
      const [rawKey, ...rest] = (negated ? term.slice(1) : term).split(":");
      const key = rawKey!.toLowerCase();
      const value = unquote(rest.join(":"));
      const holds = (() => {
        if (rest.length === 0) return ticket.title.toLowerCase().includes(term.toLowerCase());
        if (key === "is") return value === "open" || value === "closed" ? ticket.state === value : true;
        if (key === "no") return value === "assignee" || value === "owner" ? ticket.assignees.length === 0 : valuesOf(ticket, value).length === 0;
        if (key === "assignee" || key === "owner") return has(ticket.assignees, value === "@me" ? me : value);
        if (key === "author" || key === "reporter") return ticket.author?.toLowerCase() === value.toLowerCase();
        return has(valuesOf(ticket, key), value);
      })();
      return negated ? !holds : holds;
    });
}

/** Jira's JQL, clause by clause: `=`, `!=`, `in (…)`, `is EMPTY`, and the two functions a query is most often written with. */
function byJql(query: string, me: string): (ticket: Ticket) => boolean {
  const clauses = query.replace(/\s+ORDER BY.*$/i, "").split(/\s+AND\s+/i);
  return (ticket) =>
    clauses.every((clause) => {
      const one = /^\s*(\w+)\s*(!=|=|not in|in|is not|is)\s*(.+?)\s*$/i.exec(clause);
      if (one === null) return true;
      const [, field, op, rawValue] = one as unknown as [string, string, string, string];
      const operator = op.toLowerCase();
      const name = field.toLowerCase();
      if (name === "statuscategory") return (unquote(rawValue) === "Done") === (operator === "=") ? ticket.state === "closed" : ticket.state === "open";
      if (name === "assignee" && /currentUser\(\)/i.test(rawValue)) return has(ticket.assignees, me) === (operator === "=");
      const values = name === "assignee" ? ticket.assignees : valuesOf(ticket, field);
      if (/openSprints\(\)/i.test(rawValue)) return values.length > 0 && operator === "in";
      if (/^EMPTY$/i.test(rawValue)) return (values.length === 0) === (operator === "is");
      const wanted = /^\((.*)\)$/.exec(rawValue)?.[1]?.split(",").map(unquote) ?? [unquote(rawValue)];
      const any = wanted.some((it) => has(values, it));
      return operator === "=" || operator === "in" ? any : !any;
    });
}

/** The tab's tracker over a scenario's seed, built per window, so what is saved is gone with the window. */
export function issuesTabSource(seed: IssuesSeed): IssuesSource {
  const repositories = new Map(seed.map((one) => [one.tracker.manifestId, { ...one, tracker: structuredClone(one.tracker) }]));
  const of = (manifestId: string) => repositories.get(manifestId);
  const saved = (manifestId: string): SavedQuery[] => {
    const tracker = of(manifestId)?.tracker;
    return tracker?.state === "read" ? tracker.saved : [];
  };
  const keep = (manifestId: string, next: SavedQuery[]) => {
    const one = of(manifestId);
    if (one !== undefined && one.tracker.state === "read") one.tracker = { ...one.tracker, saved: next };
    return next;
  };
  const stamped = (manifestId: string, one: Ticket): Ticket => ({ ...one, holders: of(manifestId)?.held?.[one.reference] ?? [] });
  const refused = <T,>(manifestId: string): Read<T> => ({ ok: false, said: `No tracker is read for ${manifestId}.` });
  return {
    trackers: async (manifestIds) => ({ ok: true, value: [...repositories.values()].map((one) => one.tracker).filter((one) => manifestIds.includes(one.manifestId)) }),
    run: async (manifestId, query) => {
      const one = of(manifestId);
      if (one === undefined || one.tracker.state !== "read") return refused(manifestId);
      const matches = (one.language === "jql" ? byJql : byTerms)(query, one.me ?? "");
      return { ok: true, value: (one.tickets ?? []).filter(matches).map((ticket) => stamped(manifestId, ticket)) };
    },
    read: async (manifestId, reference) => {
      const ticket = of(manifestId)?.tickets?.find((it) => it.reference === reference);
      return ticket === undefined ? { ok: false, said: `${reference} is not an issue of this repository.` } : { ok: true, value: { ...ticket, ...stamped(manifestId, ticket) } };
    },
    write: async (manifestId, _words) => {
      const writes = of(manifestId)?.writes;
      // A model takes a moment; the walk sees the query being written.
      await new Promise((done) => setTimeout(done, 600));
      return writes === undefined ? refused(manifestId) : { ok: true, value: writes };
    },
    save: async (manifestId, query: QueryToSave) => {
      const was = saved(manifestId);
      const at = was.findIndex((one) => one.id === query.id);
      const kept: SavedQuery = { ...query, id: query.id ?? `${query.name}:${Date.now()}` };
      return { ok: true, value: keep(manifestId, at < 0 ? [...was, kept] : was.map((one, i) => (i === at ? kept : one))) };
    },
    remove: async (manifestId, id) => ({ ok: true, value: keep(manifestId, saved(manifestId).filter((one) => one.id !== id)) }),
  };
}
