// The mock Fleet's answers to the Reviews tab: the same shapes the real one serves (`PullReviewLists`, `PullRequestPage`
// with its threads, a file's patch, a posted review, a resolved thread), built from the mock forge's pull requests
// (`pr-review-pages.ts`). **A thread the fixture put on a hunk line is served by its place in the file**, and a draft
// posted comes back the same way, so the Bridge store meets nothing a real Fleet would not send. **An agent run is served as Fleet
// serves it**: started at once, moving on `review_run.changed`, kept for the commit it ran on and read again by `getReviewRuns`.
// What it comes to is the fixture of the pull request, converted (`review-run-fixtures.ts`). State is per window.

import { refusedWith, type PullRequestEntry, type PullRequestFilePatch, type PullRequestPage, type PullReviewLists, type PullReviewRow, type PullReviewThread, type PullThreadResolved, type ResolvePullThread, type ReviewResult, type ReviewRun, type ReviewTool, type ReviewTools, type WalkPlace, type WritePullRequest } from "@armada/protocol";

import type { ReviewsApi } from "../../../shared/api/reviews";
import type { PrList, Thread, ToolDef } from "../pr-review/model";
import { patchOfLines, placeOfLine } from "../pr-review/patch";
import { idOfName } from "../pr-review/tools";
import { unanswered } from "./moment";
import { accountResult, answerResult, findingsResult, themesOf } from "./review-run-fixtures";
import type { MockPr, ReviewSeed } from "./review-seed";
import { NO_SEARCH_OR_WALK, createMockSearchAndWalk } from "./review-walk-fleet";

/** How long a mock run takes to come to something. */
export const AGENT_MS = 2200;

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

/** Three places merged by id, the most specific winning, as Fleet's `list_review_tools` merges them. */
function resolved(layers: readonly (readonly ToolDef[])[]): ReviewTool[] {
  const merged: ReviewTool[] = [];
  for (const layer of layers) {
    for (const def of layer) {
      const tool: ReviewTool = { id: def.id, kind: def.kind, name: def.name, prompt: def.checks, source: def.source };
      const at = merged.findIndex((one) => one.id === tool.id);
      if (at < 0) merged.push(tool);
      else merged[at] = { ...tool, replaces: merged[at]!.source };
    }
  }
  return merged;
}

export type MockForge = ReviewsApi & {
  /** The next write is refused, in the forge's own sentence. */
  refuseNextWrite: (said: string) => void;
  /** The next run started fails, with this reason. */
  failNextRun: (said: string) => void;
};

export function createMockForge(seed: ReviewSeed): MockForge {
  const held = new Map<string, Held>(
    seed.prs.map((pr) => [`${pr.manifestId}#${pr.number}`, { pr, threads: pr.threads.map((one) => wireThread(pr, one)), conversation: [...pr.page.conversation], review: pr.page.review, commits: [...pr.page.commits], head: pr.head }]),
  );
  let reads = 0;
  let refusal: string | undefined;
  let failure: string | undefined;
  let ids = 0;
  const find = (manifestId: string, number: number) => held.get(`${manifestId}#${number}`);

  const kit: ToolDef[] = [...seed.tools.kit];
  const repository: Record<string, ToolDef[]> = Object.fromEntries(Object.entries(seed.tools.repository).map(([id, defs]) => [id, [...defs]]));
  const toolsOf = (manifestId: string): ReviewTools => {
    const left = seed.tools.leftOut?.[manifestId] ?? [];
    return { tools: resolved([seed.tools.armada, repository[manifestId] ?? [], kit]), ...(left.length === 0 ? {} : { left_out: left.map((one) => ({ file: one.file, source: one.source, said: one.said })) }) };
  };

  const listeners = new Set<(run: ReviewRun) => void>();
  const told = (run: ReviewRun) => listeners.forEach((one) => one({ ...run }));
  /** Every run kept or going, by pull request. A session's how it works and why, and the questions already asked, are kept from the start. */
  const runs = new Map<string, ReviewRun[]>();
  const keptAt = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();
  for (const pr of seed.prs) {
    const base = { manifest_id: pr.manifestId, number: pr.number, head: pr.head, state: "done", started_at: keptAt(30), finished_at: keptAt(29) } as const;
    const kept: ReviewRun[] = [];
    if (pr.explainedBy === "session") {
      kept.push({ ...base, kind: "explanation", name: "how-it-works", result: themesOf(pr) });
      if (pr.accounts.why !== undefined) kept.push({ ...base, kind: "explanation", name: "why", result: accountResult(pr.accounts.why) });
    }
    for (const words of pr.asked) kept.push({ ...base, kind: "ask", name: words, result: answerResult(pr, words) });
    runs.set(`${pr.manifestId}#${pr.number}`, kept);
  }

  /** What a run comes to: the fixture of its pull request, in the shape its kind has. */
  const resultOf = (pr: MockPr, kind: ReviewRun["kind"], name: string, place: WalkPlace | undefined): ReviewResult => {
    if (kind === "ask") return answerResult(pr, name);
    if (kind === "tool" && name === "walk" && place !== undefined) {
      const walk = seed.walks[pr.key];
      if (walk?.kind === "api") return { shape: "api_walk", place, requests: [...walk.requests] };
      return { shape: "web_walk", place, steps: walk?.kind === "web" ? [...walk.steps] : [] };
    }
    if (kind === "tool") return findingsResult(pr, seed.results[pr.key]?.[name] ?? []);
    return name === "how-it-works" ? themesOf(pr) : accountResult(pr.accounts[name]);
  };

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
    ...(one.pr.holder === undefined ? {} : { holder: one.pr.holder }),
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

  const searchAndWalk = createMockSearchAndWalk({
    seed,
    rowOf: (key) => {
      const one = [...held.values()].find((it) => it.pr.key === key);
      return one === undefined ? undefined : rowOf(one, false);
    },
    factsOf: (manifestId, number) => {
      const one = find(manifestId, number);
      return one === undefined ? undefined : { repository: one.pr.repository, branch: one.pr.branch, conversation: one.conversation, checks: one.pr.page.checks, files: one.pr.page.files };
    },
    told,
  });

  return {
    ...searchAndWalk,
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
    readReviewPatches: async (manifestId, number) => {
      const one = find(manifestId, number);
      if (one === undefined) return { ok: false, outcome: unanswered(`/pull_request_patches/${manifestId}/${number}`) };
      const hunks = one.pr.explanation.themes.flatMap((theme) => theme.changes.flatMap((change) => change.hunks));
      const files = [...new Set(hunks.map((hunk) => hunk.path))].map(
        (path): PullRequestFilePatch => ({
          manifest_id: manifestId,
          number,
          path,
          patch: hunks.filter((hunk) => hunk.path === path).map((hunk) => patchOfLines(hunk.lines)).join("\n"),
        }),
      );
      return { ok: true, value: { files } };
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
    listReviewTools: async (manifestId) => ({ ok: true, value: toolsOf(manifestId) }),
    saveReviewTool: async (manifestId, save) => {
      const id = idOfName(save.name);
      const path = `/review_tools/${manifestId}`;
      const refuse = (code: string, message: string) => ({ ok: false, outcome: refusedWith(422, JSON.stringify({ code, message }), { method: "POST", path }) }) as const;
      if (save.name.trim() === "" || save.prompt.trim() === "") return refuse("fleet.review_tool_blank", "A tool needs a name and a prompt");
      if (id === "") return refuse("fleet.review_tool_not_a_name", `${save.name.trim()} is no words`);
      const layer = save.scope === "kit" ? kit : (repository[manifestId] ??= []);
      const at = layer.findIndex((one) => one.id === id);
      if (at >= 0 && save.overwrite !== true) return refuse("fleet.review_tool_exists", `${id}.md is already in ${save.scope === "kit" ? "the Kit" : "this repository"}`);
      const def: ToolDef = { id, kind: save.kind, name: save.name.trim(), source: save.scope, mark: "custom", checks: save.prompt.trim(), agent: true };
      if (at >= 0) layer[at] = def;
      else layer.push(def);
      return { ok: true, value: toolsOf(manifestId) };
    },
    startReviewRun: async (manifestId, start) => {
      const one = find(manifestId, start.number);
      if (one === undefined) return { ok: false, outcome: unanswered(`/review_runs/${manifestId}`) };
      const key = `${manifestId}#${start.number}`;
      const mine = runs.get(key) ?? [];
      const kept = mine.find((it) => it.kind === start.kind && it.name === start.name);
      // A run going is answered, not started twice; a kept result is answered as it stands unless he asked for another.
      if (kept !== undefined && (kept.state === "queued" || kept.state === "running" || (kept.state === "done" && start.again !== true))) return { ok: true, value: { ...kept } };
      const run: ReviewRun = { manifest_id: manifestId, number: start.number, kind: start.kind, name: start.name, head: one.head, state: "queued", started_at: new Date().toISOString() };
      runs.set(key, [run, ...mine.filter((it) => it !== kept)]);
      const reason = failure;
      failure = undefined;
      globalThis.setTimeout(() => {
        run.state = "running";
        told(run);
      }, 0);
      globalThis.setTimeout(() => {
        run.finished_at = new Date().toISOString();
        if (reason === undefined) {
          run.state = "done";
          run.result = resultOf(one.pr, start.kind, start.name, start.place);
        } else {
          run.state = "failed";
          run.failed = reason;
        }
        told(run);
      }, AGENT_MS);
      return { ok: true, value: { ...run } };
    },
    getReviewRuns: async (manifestId, number) => {
      if (find(manifestId, number) === undefined) return { ok: false, outcome: unanswered(`/review_runs/${manifestId}/${number}`) };
      const mine = [...(runs.get(`${manifestId}#${number}`) ?? [])].sort((a, b) => b.started_at.localeCompare(a.started_at));
      return { ok: true, value: { runs: mine.map((run) => ({ ...run })) } };
    },
    onReviewRun: (onRun) => {
      listeners.add(onRun);
      return () => listeners.delete(onRun);
    },
    refuseNextWrite: (said) => {
      refusal = said;
    },
    failNextRun: (said) => {
      failure = said;
    },
  };
}

/** What a window with no mock forge answers: that nothing is behind it. */
export const NO_FORGE: ReviewsApi = {
  getReviewLists: async () => ({ ok: false, outcome: unanswered("/review_lists") }),
  readReviewPage: async (manifestId, number) => ({ ok: false, outcome: unanswered(`/pull_request_details/${manifestId}/${number}`) }),
  readReviewFile: async (manifestId, number) => ({ ok: false, outcome: unanswered(`/pull_request_files/${manifestId}/${number}`) }),
  readReviewPatches: async (manifestId, number) => ({ ok: false, outcome: unanswered(`/pull_request_patches/${manifestId}/${number}`) }),
  writeReview: async (manifestId) => ({ ok: false, outcome: unanswered(`/pull_request_writes/${manifestId}`) }),
  resolveReviewThread: async (manifestId) => ({ ok: false, outcome: unanswered(`/review_threads/${manifestId}`) }),
  listReviewTools: async (manifestId) => ({ ok: false, outcome: unanswered(`/review_tools/${manifestId}`) }),
  saveReviewTool: async (manifestId) => ({ ok: false, outcome: unanswered(`/review_tools/${manifestId}`) }),
  startReviewRun: async (manifestId) => ({ ok: false, outcome: unanswered(`/review_runs/${manifestId}`) }),
  getReviewRuns: async (manifestId, number) => ({ ok: false, outcome: unanswered(`/review_runs/${manifestId}/${number}`) }),
  onReviewRun: () => () => undefined,
  ...NO_SEARCH_OR_WALK,
};
