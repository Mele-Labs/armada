// The Reviews tab's reads and writes: the four lists across every repository Fleet serves, and one pull request
// by repository and number, whoever opened it and whether or not a Session holds it. Nothing is kept here and
// nothing polls: the renderer asks when the tab opens, on Refresh, and when a pull request is opened.

import type { Outcome, PullRequestFilePatch, PullRequestPage, PullReviewLists, PullThreadResolved, ResolvePullThread, WritePullRequest } from "@armada/protocol";

import type { ReviewActed } from "../shared/api/reviews";
import { ask, type Answer } from "./request";

const NOT_CONNECTED: Outcome = { ok: false, why: "not_connected" };

const acted = <T>(answer: Answer): ReviewActed<T> => (answer.ok === true ? { ok: true, value: answer.body as T } : { ok: false, outcome: answer.outcome });

export class ReviewsHost {
  private readonly port: () => number | null;

  constructor(port: () => number | null) {
    this.port = port;
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

  /** A review, posted as one. Fleet answers the page read again; a refusal carries the forge's own sentence. */
  async write(manifestId: string, write: WritePullRequest): Promise<ReviewActed<PullRequestPage>> {
    const port = this.port();
    return port === null ? { ok: false, outcome: NOT_CONNECTED } : acted(await ask(port, "POST", `/pull_request_writes/${encodeURIComponent(manifestId)}`, write));
  }

  async resolve(manifestId: string, resolve: ResolvePullThread): Promise<ReviewActed<PullThreadResolved>> {
    const port = this.port();
    return port === null ? { ok: false, outcome: NOT_CONNECTED } : acted(await ask(port, "POST", `/review_threads/${encodeURIComponent(manifestId)}`, resolve));
  }
}
