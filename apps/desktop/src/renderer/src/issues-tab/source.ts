// What the Issues store reads and writes, over the calls Fleet answers (`shared/api/issues.ts`). **The store holds no
// `window.armada`**, so a test hands it a fake one. Fleet's words for a refusal, including a tracker's own for a query it will
// not run, reach the tab as they are.

import type { IssueQueries, IssueRead, Outcome, TrackedIssue } from "@armada/protocol";
import { refusalWords } from "@armada/screens/src/refusal-words";

import type { IssueQueryActed, IssuesApi } from "../../../shared/api/issues";
import type { IssuesSource, Read, SavedQuery, Ticket, TicketRead, Tracker } from "./model";

/** The program a refusal named as missing, in the field Fleet carries it in. */
const missingOf = (outcome: Outcome): string | undefined => {
  const tool = outcome.ok === false && outcome.why === "refused" ? outcome.error.fields?.missing_tool : undefined;
  return typeof tool === "string" && tool !== "" ? tool : undefined;
};

const worded = <T>(answer: IssueQueryActed<T>): Read<T> => {
  if (answer.ok) return { ok: true, value: answer.value };
  const missing = missingOf(answer.outcome);
  return { ok: false, said: refusalWords(answer.outcome), ...(missing === undefined ? {} : { missing }) };
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

/** The source over what the preload (or the mock Fleet) offers. */
export const issuesSourceOf = (api: IssuesApi): IssuesSource => ({
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
