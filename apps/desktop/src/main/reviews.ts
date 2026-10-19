// The Reviews tab's reads and writes: the four lists across every repository Fleet serves, and one pull request
// by repository and number, whoever opened it and whether or not a Session holds it. Nothing is kept here and
// nothing polls: the renderer asks when the tab opens, on Refresh, and when a pull request is opened.

import type {
  ForgetReviewSearch,
  OpenReviewWalk,
  Outcome,
  PullRequestFilePatch,
  PullRequestFilePatches,
  PullRequestPage,
  PullReviewLists,
  PullThreadResolved,
  RememberedPlace,
  RememberedPlaces,
  ResolvePullThread,
  ReviewRun,
  ReviewRuns,
  ReviewTools,
  ReviewWalk,
  SaveReviewTool,
  SaveWalkPlace,
  StartReviewRun,
  StartReviewSearch,
  WalkPlaceFound,
  WritePullRequest,
} from "@armada/protocol";

import type { ReviewActed } from "../shared/api/reviews";
import { ask, type Answer } from "./request";

const NOT_CONNECTED: Outcome = { ok: false, why: "not_connected" };

const acted = <T>(answer: Answer): ReviewActed<T> => (answer.ok === true ? { ok: true, value: answer.body as T } : { ok: false, outcome: answer.outcome });

export class ReviewsHost {
  private readonly port: () => number | null;

  private told: (run: ReviewRun) => void = () => {};
  private toldWalk: (walk: ReviewWalk) => void = () => {};

  constructor(port: () => number | null) {
    this.port = port;
  }

  /** Say where a moved run goes: main's, which owns the windows. */
  onRun(told: (run: ReviewRun) => void): void {
    this.told = told;
  }

  /** `review_run.changed`. Nothing is kept: each window holds the runs it drew. */
  changed(run: ReviewRun): void {
    this.told(run);
  }

  /** Say where a moved walk goes: main's, which owns the windows. */
  onWalk(told: (walk: ReviewWalk) => void): void {
    this.toldWalk = told;
  }

  /** `review_walk.changed`. Nothing is kept either. */
  walkChanged(walk: ReviewWalk): void {
    this.toldWalk(walk);
  }

  async lists(): Promise<ReviewActed<PullReviewLists>> {
    const port = this.port();
    return port === null ? { ok: false, outcome: NOT_CONNECTED } : acted(await ask(port, "GET", "/review_lists"));
  }

  async page(manifestId: string, number: number): Promise<ReviewActed<PullRequestPage>> {
    const port = this.port();
    return port === null ? { ok: false, outcome: NOT_CONNECTED } : acted(await ask(port, "GET", `/pull_request_details/${encodeURIComponent(manifestId)}/${number}`));
  }

  async file(manifestId: string, number: number, path: string): Promise<ReviewActed<PullRequestFilePatch>> {
    const port = this.port();
    return port === null
      ? { ok: false, outcome: NOT_CONNECTED }
      : acted(await ask(port, "GET", `/pull_request_files/${encodeURIComponent(manifestId)}/${number}?path=${encodeURIComponent(path)}`));
  }

  async patches(manifestId: string, number: number): Promise<ReviewActed<PullRequestFilePatches>> {
    const port = this.port();
    return port === null ? { ok: false, outcome: NOT_CONNECTED } : acted(await ask(port, "GET", `/pull_request_patches/${encodeURIComponent(manifestId)}/${number}`));
  }

  /** A review, posted as one. Fleet answers the page read again; a refusal carries the forge's own sentence. */
  async write(manifestId: string, write: WritePullRequest): Promise<ReviewActed<PullRequestPage>> {
    const port = this.port();
    return port === null ? { ok: false, outcome: NOT_CONNECTED } : acted(await ask(port, "POST", `/pull_request_writes/${encodeURIComponent(manifestId)}`, write));
  }

  async resolve(manifestId: string, resolve: ResolvePullThread): Promise<ReviewActed<PullThreadResolved>> {
    const port = this.port();
    return port === null ? { ok: false, outcome: NOT_CONNECTED } : acted(await ask(port, "POST", `/review_threads/${encodeURIComponent(manifestId)}`, resolve));
  }

  async tools(manifestId: string): Promise<ReviewActed<ReviewTools>> {
    const port = this.port();
    return port === null ? { ok: false, outcome: NOT_CONNECTED } : acted(await ask(port, "GET", `/review_tools/${encodeURIComponent(manifestId)}`));
  }

  async saveTool(manifestId: string, save: SaveReviewTool): Promise<ReviewActed<ReviewTools>> {
    const port = this.port();
    return port === null ? { ok: false, outcome: NOT_CONNECTED } : acted(await ask(port, "POST", `/review_tools/${encodeURIComponent(manifestId)}`, save));
  }

  /** Answers at once; the run's progress and result arrive as `review_run.changed`. */
  async startRun(manifestId: string, start: StartReviewRun): Promise<ReviewActed<ReviewRun>> {
    const port = this.port();
    return port === null ? { ok: false, outcome: NOT_CONNECTED } : acted(await ask(port, "POST", `/review_runs/${encodeURIComponent(manifestId)}`, start));
  }

  async runs(manifestId: string, number: number): Promise<ReviewActed<ReviewRuns>> {
    const port = this.port();
    return port === null ? { ok: false, outcome: NOT_CONNECTED } : acted(await ask(port, "GET", `/review_runs/${encodeURIComponent(manifestId)}/${number}`));
  }

  /** Natural Search. Answers at once; the run's progress and result arrive as `review_run.changed`, a run of kind `search`. */
  async startSearch(start: StartReviewSearch): Promise<ReviewActed<ReviewRun>> {
    const port = this.port();
    return port === null ? { ok: false, outcome: NOT_CONNECTED } : acted(await ask(port, "POST", "/review_searches", start));
  }

  async searches(): Promise<ReviewActed<ReviewRuns>> {
    const port = this.port();
    return port === null ? { ok: false, outcome: NOT_CONNECTED } : acted(await ask(port, "GET", "/review_searches"));
  }

  async forgetSearch(forget: ForgetReviewSearch): Promise<ReviewActed<ReviewRuns>> {
    const port = this.port();
    return port === null ? { ok: false, outcome: NOT_CONNECTED } : acted(await ask(port, "POST", "/review_searches/forget", forget));
  }

  async findPlace(manifestId: string, number: number): Promise<ReviewActed<WalkPlaceFound>> {
    const port = this.port();
    return port === null ? { ok: false, outcome: NOT_CONNECTED } : acted(await ask(port, "GET", `/walk_finds/${encodeURIComponent(manifestId)}/${number}`));
  }

  async places(): Promise<ReviewActed<RememberedPlaces>> {
    const port = this.port();
    return port === null ? { ok: false, outcome: NOT_CONNECTED } : acted(await ask(port, "GET", "/walk_places"));
  }

  async savePlace(manifestId: string, save: SaveWalkPlace): Promise<ReviewActed<RememberedPlace>> {
    const port = this.port();
    return port === null ? { ok: false, outcome: NOT_CONNECTED } : acted(await ask(port, "POST", `/walk_places/${encodeURIComponent(manifestId)}`, save));
  }

  async forgetPlace(manifestId: string, id: string): Promise<ReviewActed<RememberedPlaces>> {
    const port = this.port();
    return port === null ? { ok: false, outcome: NOT_CONNECTED } : acted(await ask(port, "POST", `/walk_places/${encodeURIComponent(manifestId)}/${encodeURIComponent(id)}/forget`));
  }

  /** Answers at once; progress arrives as `review_walk.changed`. */
  async openWalk(manifestId: string, open: OpenReviewWalk): Promise<ReviewActed<ReviewWalk>> {
    const port = this.port();
    return port === null ? { ok: false, outcome: NOT_CONNECTED } : acted(await ask(port, "POST", `/review_walks/${encodeURIComponent(manifestId)}`, open));
  }

  async closeWalk(manifestId: string, id: string): Promise<ReviewActed<ReviewWalk>> {
    const port = this.port();
    return port === null ? { ok: false, outcome: NOT_CONNECTED } : acted(await ask(port, "POST", `/review_walks/${encodeURIComponent(manifestId)}/${encodeURIComponent(id)}/close`));
  }
}
