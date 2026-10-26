// A repository's open issues, as main asks Fleet for them: one request, and nothing kept here, since
// Fleet holds the list. #91. And the Autopilot that works them (#2092), which Fleet holds too.

import type { Autopilot, AutopilotList, AutopilotProposal, InstalledTool, IssueList, IssueQueries, IssueRead, Outcome, PullRequestState, SaveIssueQuery, SavedIssueQuery, WrittenIssueQuery } from "@armada/protocol";
import type { AutopilotActed } from "../shared/api/autopilot";
import type { IssueNumberRead, IssueQueryActed, IssuesRead, PullRequestNumberRead } from "../shared/api/issues";
import { ask, MODEL_CALL_MS } from "./request";

const NOT_CONNECTED: Outcome = { ok: false, why: "not_connected" };

export class IssuesHost {
  private readonly port: () => number | null;
  private told: (autopilot: Autopilot) => void = () => {};

  constructor(port: () => number | null) {
    this.port = port;
  }

  async list(manifestId: string, refresh: boolean): Promise<IssuesRead> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const path = `/issues/${encodeURIComponent(manifestId)}${refresh ? "?refresh=true" : ""}`;
    const answer = await ask(port, "GET", path);
    if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
    return { ok: true, value: answer.body as IssueList };
  }

  /** `GET /issues/:repository/:reference`: one issue, open or closed, named as its tracker writes it. */
  async issue(manifestId: string, reference: string): Promise<IssueNumberRead> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const answer = await ask(port, "GET", `/issues/${encodeURIComponent(manifestId)}/${encodeURIComponent(reference)}`);
    return answer.ok === true ? { ok: true, value: answer.body as IssueRead } : { ok: false, outcome: answer.outcome };
  }

  /** One of the tracker query routes, which sit under `/issue_queries` so a reference can never be read as one of them. */
  private async query<T>(method: "GET" | "POST", manifestId: string, rest: string, body?: unknown, slow = false): Promise<IssueQueryActed<T>> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const path = `/issue_queries/${encodeURIComponent(manifestId)}${rest}`;
    const answer = slow ? await ask(port, method, path, body, MODEL_CALL_MS) : await ask(port, method, path, body);
    return answer.ok === true ? { ok: true, value: answer.body as T } : { ok: false, outcome: answer.outcome };
  }

  /** `POST /issue_queries/:repository/search`: one query run, nothing kept and no model asked. */
  searchIssues(manifestId: string, query: string): Promise<IssueQueryActed<IssueList>> {
    return this.query("POST", manifestId, "/search", { query });
  }

  /** `GET /issue_queries/:repository`: the tracker's guide and the saved queries. */
  issueQueries(manifestId: string): Promise<IssueQueryActed<IssueQueries>> {
    return this.query("GET", manifestId, "");
  }

  /** `POST /issue_queries/:repository/write`: a model call, so it waits as one does. */
  writeIssueQuery(manifestId: string, words: string): Promise<IssueQueryActed<WrittenIssueQuery>> {
    return this.query("POST", manifestId, "/write", { words }, true);
  }

  /** `POST /issue_queries/:repository`: keeps a query, or edits the one its `id` names. */
  saveIssueQuery(manifestId: string, save: SaveIssueQuery): Promise<IssueQueryActed<SavedIssueQuery>> {
    return this.query("POST", manifestId, "", save);
  }

  /** `POST /issue_queries/:repository/:id/forget`. */
  forgetIssueQuery(manifestId: string, id: string): Promise<IssueQueryActed<IssueQueries>> {
    return this.query("POST", manifestId, `/${encodeURIComponent(id)}/forget`);
  }

  /** `POST /issue_queries/:repository/install_tool`: installs what a refusal's `missing_tool` named. Brew can take a while. */
  installIssueTool(manifestId: string, tool: string): Promise<IssueQueryActed<InstalledTool>> {
    return this.query("POST", manifestId, "/install_tool", { tool }, true);
  }

  /** `GET /pull_requests/:repository/:number`: one pull request by the repository's Manifest, with no Session. */
  async pullRequest(manifestId: string, number: number): Promise<PullRequestNumberRead> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const answer = await ask(port, "GET", `/pull_requests/${encodeURIComponent(manifestId)}/${number}`);
    return answer.ok === true ? { ok: true, value: answer.body as PullRequestState } : { ok: false, outcome: answer.outcome };
  }

  /** `POST /pull_requests/:repository/:number/refresh`: the forge read now; the change itself arrives as events. */
  async refreshPullRequest(manifestId: string, number: number): Promise<PullRequestNumberRead> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const answer = await ask(port, "POST", `/pull_requests/${encodeURIComponent(manifestId)}/${number}/refresh`);
    return answer.ok === true ? { ok: true, value: answer.body as PullRequestState } : { ok: false, outcome: answer.outcome };
  }

  /** One Autopilot act. A sentence read and a set-aside pick dispatched are model calls, so they wait longer. */
  async autopilot<T = Autopilot>(method: "GET" | "POST", path: string, body?: unknown, slow = false): Promise<AutopilotActed<T>> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const answer = slow ? await ask(port, method, path, body, MODEL_CALL_MS) : await ask(port, method, path, body);
    if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
    return { ok: true, value: answer.body as T };
  }

  /** The path of one repository's Autopilot, and of something under it. */
  static autopilotOf(manifestId: string, rest = ""): string {
    return `/autopilots/${encodeURIComponent(manifestId)}${rest}`;
  }

  async proposeAutopilot(manifestId: string, sentence: string, localNow: string): Promise<AutopilotActed<AutopilotProposal>> {
    const body = { sentence, local_now: localNow };
    return await this.autopilot<AutopilotProposal>("POST", IssuesHost.autopilotOf(manifestId, "/proposal"), body, true);
  }

  /** Say where a changed Autopilot goes: main's, which owns the windows. */
  onAutopilotChanged(told: (autopilot: Autopilot) => void): void {
    this.told = told;
  }

  /** `autopilot.changed`, carried whole. */
  autopilotChanged(autopilot: Autopilot): void {
    this.told(autopilot);
  }

  /** Every Autopilot again, on a resync: one changed while the stream was down was never told. */
  async autopilotsAgain(): Promise<void> {
    const read = await this.autopilot<AutopilotList>("GET", "/autopilots");
    if (read.ok) read.value.autopilots.forEach((one) => this.told(one));
  }
}
