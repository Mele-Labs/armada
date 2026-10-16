// The mock Fleet's answers to the Reviews tab: the same shapes the real one serves (`PullReviewLists`, `PullRequestPage`
// with its threads, a file's patch, a posted review, a resolved thread), built from the mock forge's pull requests
// (`pr-review-pages.ts`). **A thread the fixture put on a hunk line is served by its place in the file**, and a draft
// posted comes back the same way, so the Bridge store meets nothing a real Fleet would not send. State is per window.

import { refusedWith, type PullRequestEntry, type PullRequestFilePatch, type PullRequestPage, type PullReviewLists, type PullReviewRow, type PullReviewThread, type PullThreadResolved, type ResolvePullThread, type WritePullRequest } from "@armada/protocol";

import type { ReviewsApi } from "../../../shared/api/reviews";
import type { AgentFixtures, MockPr, PrList, ReviewSeed, Thread } from "../pr-review/model";
import { patchOfLines, placeOfLine } from "../pr-review/patch";
import { unanswered } from "./moment";

const LISTS: readonly PrList[] = ["requested", "team", "assigned", "mine"];

type Held = {
  pr: MockPr;
  threads: Thread[];
  conversation: PullRequestEntry[];
  review: PullRequestPage["review"];
  commits: PullRequestPage["commits"];
  head: string;
};

/** The thread a fixture wrote on a line of a hunk, as the forge names it: by path, line and side. */
function wireThread(pr: MockPr, one: MockPr["threads"][number]): Thread {
  const change = pr.explanation.themes.flatMap((theme) => theme.changes).find((at) => at.id === one.change);
  const hunk = change?.hunks[one.hunk];
  const place = hunk === undefined || one.line === undefined ? undefined : placeOfLine(hunk.lines, one.line);
  return {
    id: one.id,
    path: hunk?.path ?? change?.title ?? "",
    ...(place === undefined ? {} : { line: place.line }),
    side: place?.side ?? "right",
    resolved: one.resolved,
    comments: one.comments,
  };
}

/** What the agent half of the store is driven by: the seed's tools and results, and what each pull request's agent would say. */
export const agentOf = (seed: ReviewSeed): AgentFixtures => ({
  built: true,
  tools: seed.tools,
  results: seed.results,
  finds: seed.finds,
  prs: Object.fromEntries(
    seed.prs.map((pr) => [pr.key, { explanation: pr.explanation, explainedBy: pr.explainedBy, accounts: pr.accounts, answers: pr.answers, asked: pr.asked, ...(pr.holder === undefined ? {} : { holder: pr.holder }) }]),
  ),
});

export type MockForge = ReviewsApi & {
  /** The next write is refused, in the forge's own sentence. */
  refuseNextWrite: (said: string) => void;
};

export function createMockForge(seed: ReviewSeed): MockForge {
  const held = new Map<string, Held>(
    seed.prs.map((pr) => [`${pr.manifestId}#${pr.number}`, { pr, threads: pr.threads.map((one) => wireThread(pr, one)), conversation: [...pr.page.conversation], review: pr.page.review, commits: [...pr.page.commits], head: pr.head }]),
  );
  let reads = 0;
  let refusal: string | undefined;
  let ids = 0;
  const find = (manifestId: string, number: number) => held.get(`${manifestId}#${number}`);

  const rowOf = (one: Held, team: boolean): PullReviewRow => ({
    manifest_id: one.pr.manifestId,
    repository: one.pr.repository,
    number: one.pr.number,
    title: one.pr.title,
    branch: one.pr.branch,
    address: one.pr.address,
    draft: one.pr.draft,
    checks: one.pr.checks,
    review: one.review,
    head: one.head,
    author: one.pr.page.author ?? "",
    updated_at: one.pr.updatedAt,
    ...(team && one.pr.team !== undefined ? { team: one.pr.team } : {}),
  });

  const pageOf = (one: Held): PullRequestPage => ({
    ...one.pr.page,
    review: one.review,
    conversation: one.conversation,
    commits: one.commits,
    head: one.head,
    body: one.pr.body,
    ...(one.threads.length === 0 ? {} : { threads: one.threads }),
  });

  return {
    getReviewLists: async () => {
      reads += 1;
      // A commit pushed since he first read the list arrives with the second read.
      if (reads === 2) {
        for (const [key, push] of Object.entries(seed.pushes)) {
          const one = [...held.values()].find((it) => it.pr.key === key);
          if (one === undefined) continue;
          one.head = push.sha;
          one.commits = [...one.commits, { sha: push.sha, headline: push.headline, author: push.author, at: new Date().toISOString() }];
        }
      }
      const rows = (list: PrList) => [...held.values()].filter((one) => one.pr.lists.includes(list)).map((one) => rowOf(one, list === "team"));
      const [requested, team, assigned, mine] = LISTS.map(rows) as [PullReviewRow[], PullReviewRow[], PullReviewRow[], PullReviewRow[]];
      const value: PullReviewLists = { me: seed.me, fetched_at: new Date().toISOString(), requested, team, assigned, mine };
      return { ok: true, value };
    },
    readReviewPage: async (manifestId, number) => {
      const one = find(manifestId, number);
      return one === undefined ? { ok: false, outcome: unanswered(`/pull_request_details/${manifestId}/${number}`) } : { ok: true, value: pageOf(one) };
    },
    readReviewFile: async (manifestId, number, path) => {
      const one = find(manifestId, number);
      const hunks = one?.pr.explanation.themes.flatMap((theme) => theme.changes.flatMap((change) => change.hunks)).filter((hunk) => hunk.path === path) ?? [];
      if (one === undefined || hunks.length === 0) return { ok: false, outcome: unanswered(`/pull_request_files/${manifestId}/${number}`) };
      const value: PullRequestFilePatch = { manifest_id: manifestId, number, path, patch: hunks.map((hunk) => patchOfLines(hunk.lines)).join("\n") };
      return { ok: true, value };
    },
    writeReview: async (manifestId, write: WritePullRequest) => {
      const one = find(manifestId, write.pull_request);
      if (one === undefined) return { ok: false, outcome: unanswered(`/pull_request_writes/${manifestId}`) };
      if (refusal !== undefined) {
        const said = refusal;
        refusal = undefined;
        return { ok: false, outcome: refusedWith(409, JSON.stringify({ code: "fleet.pull_request_write_refused", message: said }), { method: "POST", path: `/pull_request_writes/${manifestId}` }) };
      }
      const at = new Date().toISOString();
      for (const comment of write.comments ?? []) {
        const said = { id: `mock:c${(ids += 1)}`, author: seed.me, at, body: comment.body };
        if (comment.kind === "reply") {
          one.threads = one.threads.map((thread) => (thread.id === comment.thread ? { ...thread, comments: [...thread.comments, said] } : thread));
        } else {
          const line = comment.kind === "line" ? { line: comment.line } : {};
          one.threads = [...one.threads, { id: `mock:t${(ids += 1)}`, path: comment.path, ...line, side: comment.kind === "line" ? comment.side : "right", resolved: false, comments: [said] }];
        }
      }
      const verdict = write.kind === "approve" ? "approved" : write.kind === "request_changes" ? "changes_requested" : "commented";
      if (write.body !== undefined || verdict !== "commented") one.conversation = [...one.conversation, { kind: "review", author: seed.me, at, verdict, body: write.body ?? "" }];
      if (write.kind === "approve") one.review = "approved";
      if (write.kind === "request_changes") one.review = "changes_requested";
      return { ok: true, value: pageOf(one) };
    },
    resolveReviewThread: async (manifestId, resolve: ResolvePullThread) => {
      const one = [...held.values()].find((it) => it.pr.manifestId === manifestId && it.threads.some((thread) => thread.id === resolve.thread));
      if (one === undefined) return { ok: false, outcome: unanswered(`/review_threads/${manifestId}`) };
      one.threads = one.threads.map((thread: PullReviewThread) => (thread.id === resolve.thread ? { ...thread, resolved: resolve.resolved } : thread));
      const value: PullThreadResolved = { thread: resolve.thread, resolved: resolve.resolved };
      return { ok: true, value };
    },
    refuseNextWrite: (said) => {
      refusal = said;
    },
  };
}

/** What a window with no mock forge answers: that nothing is behind it. */
export const NO_FORGE: ReviewsApi = {
  getReviewLists: async () => ({ ok: false, outcome: unanswered("/review_lists") }),
  readReviewPage: async (manifestId, number) => ({ ok: false, outcome: unanswered(`/pull_request_details/${manifestId}/${number}`) }),
  readReviewFile: async (manifestId, number) => ({ ok: false, outcome: unanswered(`/pull_request_files/${manifestId}/${number}`) }),
  writeReview: async (manifestId) => ({ ok: false, outcome: unanswered(`/pull_request_writes/${manifestId}`) }),
  resolveReviewThread: async (manifestId) => ({ ok: false, outcome: unanswered(`/review_threads/${manifestId}`) }),
};
