// What the Issues store reads and writes, over the calls Fleet answers (`shared/api/issues.ts`). **The store holds no
// `window.armada`**, so a test hands it a fake one. Fleet's words for a refusal, including a tracker's own for a query it will
// not run, reach the tab as they are.

import type { BugReport, IssueQueries, IssueRead, Outcome, TrackedIssue } from "@armada/protocol";
import { refusalWords } from "@armada/screens/src/refusal-words";

import type { BugReportActed, BugReportsApi } from "../../../shared/api/bug-reports";
import type { IssueQueryActed, IssuesApi } from "../../../shared/api/issues";
import type { IssuesSource, Read, ReportedBug, SavedQuery, Ticket, TicketRead, Tracker } from "./model";

/** The program a refusal named in `field`: `missing_tool` where it is not installed, `needs_setup` where it has no project. */
const namedIn = (outcome: Outcome, field: "missing_tool" | "needs_setup"): string | undefined => {
  const tool = outcome.ok === false && outcome.why === "refused" ? outcome.error.fields?.[field] : undefined;
  return typeof tool === "string" && tool !== "" ? tool : undefined;
};

const worded = <T>(answer: IssueQueryActed<T> | BugReportActed<T>): Read<T> => {
  if (answer.ok) return { ok: true, value: answer.value };
  const missing = namedIn(answer.outcome, "missing_tool");
  const needsSetup = namedIn(answer.outcome, "needs_setup");
  return { ok: false, said: refusalWords(answer.outcome), ...(missing === undefined ? {} : { missing }), ...(needsSetup === undefined ? {} : { needsSetup }) };
};

const ticketOf = (one: TrackedIssue): Ticket => ({
  reference: one.reference,
  title: one.title,
  url: one.url,
  state: one.state,
  ...(one.author === undefined ? {} : { author: one.author }),
  assignees: one.assignees,
  updated_at: one.updated_at,
  excerpt: one.excerpt,
  fields: one.fields,
  linked_pull_request: one.linked_pull_request,
  holders: one.holders,
});

/** An issue read whole. The row's own start-of-body and last edit are not on that read, so the body and the opening stand in. */
const readOf = (one: IssueRead): TicketRead => ({
  reference: one.reference,
  title: one.title,
  url: one.url,
  state: one.state,
  ...(one.author === "" ? {} : { author: one.author }),
  assignees: one.assignees,
  updated_at: one.opened_at,
  excerpt: "",
  fields: one.fields,
  linked_pull_request: one.linked_pull_requests.length > 0,
  holders: one.holders,
  body: one.body,
  linked_pull_requests: one.linked_pull_requests.map((pull) => ({ number: pull.number, title: pull.title, address: pull.url, state: pull.state })),
});

/** A tracker nothing reads says why; one that can be read has its guide's two queries and the ones he saved. */
function trackerOf(queries: IssueQueries): Tracker {
  if (queries.guide === undefined) return { manifestId: queries.manifest_id, state: "refused", said: queries.refused ?? "The tracker would not answer." };
  return { manifestId: queries.manifest_id, state: "read", name: queries.guide.language, open: queries.guide.open, assigned: queries.guide.assigned_to_me, saved: queries.saved };
}

const savedOf = (answer: IssueQueryActed<IssueQueries>): Read<SavedQuery[]> => {
  const said = worded(answer);
  return said.ok ? { ok: true, value: said.value.saved } : said;
};

/** A report as the tab draws it: whether the owner kept it is whether Fleet has a time for that. */
export const reportedBugOf = (one: BugReport): ReportedBug => {
  const { kept, ...rest } = one;
  return { ...rest, kept: kept !== undefined };
};

/** The reported bugs over what the preload (or the mock Fleet) offers. */
export const reportsSourceOf = (api: BugReportsApi): NonNullable<IssuesSource["reports"]> => ({
  list: async () => {
    const answer = worded(await api.listBugReports());
    return answer.ok ? { ok: true, value: answer.value.reports.map(reportedBugOf) } : answer;
  },
  keep: async (id) => {
    const answer = worded(await api.keepBugReport(id));
    return answer.ok ? { ok: true, value: reportedBugOf(answer.value) } : answer;
  },
  drop: async (id) => {
    const answer = worded(await api.dropBugReport(id));
    return answer.ok ? { ok: true, value: undefined } : answer;
  },
  follow: (onChanged) => api.onBugReportsChanged((changed) => onChanged(changed.reports.map(reportedBugOf))),
});

/** The source over what the preload (or the mock Fleet) offers. The reported bugs come from the same Fleet where `bugs` is handed. */
export const issuesSourceOf = (api: IssuesApi, bugs?: BugReportsApi): IssuesSource => ({
  ...(bugs === undefined ? {} : { reports: reportsSourceOf(bugs) }),
  trackers: async (manifestIds) => {
    const answers = await Promise.all(manifestIds.map(async (id) => ({ id, answer: worded(await api.listIssueQueries(id)) })));
    const read = answers.flatMap(({ answer }) => (answer.ok ? [trackerOf(answer.value)] : []));
    const failed = answers.find(({ answer }) => !answer.ok)?.answer;
    // A repository Fleet does not serve is left out; Fleet not answering at all is said.
    return read.length === 0 && failed !== undefined && !failed.ok ? failed : { ok: true, value: read };
  },
  run: async (manifestId, query) => {
    const answer = worded(await api.searchIssues(manifestId, query));
    return answer.ok ? { ok: true, value: answer.value.issues.map(ticketOf) } : answer;
  },
  read: async (manifestId, reference) => {
    const answer = worded(await api.getIssue(manifestId, reference));
    return answer.ok ? { ok: true, value: readOf(answer.value) } : answer;
  },
  install: async (manifestId, tool) => {
    const answer = worded(await api.installIssueTool(manifestId, tool));
    if (!answer.ok) return answer;
    return answer.value.done ? { ok: true, value: undefined } : { ok: false, said: answer.value.said };
  },
  write: async (manifestId, words) => {
    const answer = worded(await api.writeIssueQuery(manifestId, words));
    return answer.ok ? { ok: true, value: { query: answer.value.query, row_fields: answer.value.row_fields, tickets: answer.value.issues.map(ticketOf) } } : answer;
  },
  save: async (manifestId, query) => {
    const kept = worded(await api.saveIssueQuery(manifestId, query));
    if (!kept.ok) return kept;
    return savedOf(await api.listIssueQueries(manifestId));
  },
  remove: async (manifestId, id) => savedOf(await api.forgetIssueQuery(manifestId, id)),
});
