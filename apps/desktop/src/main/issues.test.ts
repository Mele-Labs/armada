// One issue and one pull request by number, asked of Fleet by the repository's Manifest: the path each is
// read on, and what comes back, answered or refused.

import { once } from "node:events";
import { createServer, type Server } from "node:http";
import type { AddressInfo } from "node:net";

import { afterEach, describe, expect, it } from "vitest";

import { IssuesHost } from "./issues";

const MANIFEST = "01M2ARMADA0000000000000000";

const servers: Server[] = [];
afterEach(() => {
  while (servers.length > 0) servers.pop()?.close();
});

/** A Fleet answering every request with `status` and `body`, remembering what was asked. */
async function serving(status: number, body: unknown): Promise<{ port: number; asked: string[] }> {
  const asked: string[] = [];
  const server = createServer((request, response) => {
    asked.push(`${request.method} ${request.url}`);
    response.writeHead(status, { "content-type": "application/json" });
    response.end(JSON.stringify(body));
  });
  server.listen(0, "127.0.0.1");
  await once(server, "listening");
  servers.push(server);
  return { port: (server.address() as AddressInfo).port, asked };
}

describe("an issue by reference", () => {
  it("is read on the repository's path and answered whole", async () => {
    const issue = { reference: "#61", title: "Doctor reports a stale pid as running", url: "https://forge.invalid/issues/61", state: "closed" };
    const fleet = await serving(200, issue);
    const read = await new IssuesHost(() => fleet.port).issue(MANIFEST, "#61");
    expect(read).toEqual({ ok: true, value: issue });
    expect(fleet.asked).toEqual([`GET /issues/${MANIFEST}/%2361`]);
  });

  it("is refused in Fleet's own words where there is no such issue", async () => {
    const fleet = await serving(404, { code: "fleet.no_such_issue", message: "There is no issue 9999", run_id: "r", fields: {}, chain: [] });
    const read = await new IssuesHost(() => fleet.port).issue(MANIFEST, "9999");
    expect(read).toMatchObject({ ok: false, outcome: { ok: false, why: "refused", error: { code: "fleet.no_such_issue" } } });
  });

  it("is not asked where Fleet is not connected", async () => {
    expect(await new IssuesHost(() => null).issue(MANIFEST, "ABC-61")).toEqual({ ok: false, outcome: { ok: false, why: "not_connected" } });
  });
});

describe("a pull request by number", () => {
  it("is read by the repository's Manifest with no Session, open, merged or closed", async () => {
    const pull = { manifest_id: MANIFEST, number: 2207, state: "merged", auto_merge: false, checks: { state: "passed" }, title: "Stop the Drone count drifting", branch: "fix/drone-count", address: "https://forge.invalid/pull/2207" };
    const fleet = await serving(200, pull);
    const read = await new IssuesHost(() => fleet.port).pullRequest(MANIFEST, 2207);
    expect(read).toEqual({ ok: true, value: pull });
    expect(fleet.asked).toEqual([`GET /pull_requests/${MANIFEST}/2207`]);
  });

  it("is refused in Fleet's words where the forge cannot be read", async () => {
    const fleet = await serving(500, { code: "fleet.pull_request_unreadable", message: "The forge did not answer", run_id: "r", fields: {}, chain: [] });
    const read = await new IssuesHost(() => fleet.port).pullRequest(MANIFEST, 2207);
    expect(read).toMatchObject({ ok: false, outcome: { ok: false, why: "refused", error: { code: "fleet.pull_request_unreadable" } } });
  });

  it("is not asked where Fleet is not connected", async () => {
    expect(await new IssuesHost(() => null).pullRequest(MANIFEST, 2207)).toEqual({ ok: false, outcome: { ok: false, why: "not_connected" } });
  });
});

describe("a pull request refreshed", () => {
  it("is posted on the repository's path and answered as a read is", async () => {
    const pull = { manifest_id: MANIFEST, number: 2207, state: "open", auto_merge: false, checks: { state: "failed", failing: [] }, title: "Stop the Drone count drifting", branch: "fix/drone-count", address: "https://forge.invalid/pull/2207" };
    const fleet = await serving(200, pull);
    const read = await new IssuesHost(() => fleet.port).refreshPullRequest(MANIFEST, 2207);
    expect(read).toEqual({ ok: true, value: pull });
    expect(fleet.asked).toEqual([`POST /pull_requests/${MANIFEST}/2207/refresh`]);
  });

  it("is refused in Fleet's words where the forge cannot be read", async () => {
    const fleet = await serving(500, { code: "fleet.pull_request_unreadable", message: "The forge did not answer", run_id: "r", fields: {}, chain: [] });
    const read = await new IssuesHost(() => fleet.port).refreshPullRequest(MANIFEST, 2207);
    expect(read).toMatchObject({ ok: false, outcome: { ok: false, why: "refused", error: { code: "fleet.pull_request_unreadable" } } });
  });

  it("is not sent where Fleet is not connected", async () => {
    expect(await new IssuesHost(() => null).refreshPullRequest(MANIFEST, 2207)).toEqual({ ok: false, outcome: { ok: false, why: "not_connected" } });
  });
});

describe("a repository's issue queries", () => {
  it("are asked on the routes under /issue_queries, a body where one is sent, and the id percent-encoded", async () => {
    const fleet = await serving(200, {});
    const host = new IssuesHost(() => fleet.port);
    await host.searchIssues(MANIFEST, "is:open");
    await host.issueQueries(MANIFEST);
    await host.writeIssueQuery(MANIFEST, "open bugs");
    await host.saveIssueQuery(MANIFEST, { name: "Bugs", words: "", query: "label:bug", row_fields: [] });
    await host.forgetIssueQuery(MANIFEST, "a b/c");
    await host.installIssueTool(MANIFEST, "bd");
    expect(fleet.asked).toEqual([
      `POST /issue_queries/${MANIFEST}/search`,
      `GET /issue_queries/${MANIFEST}`,
      `POST /issue_queries/${MANIFEST}/write`,
      `POST /issue_queries/${MANIFEST}`,
      `POST /issue_queries/${MANIFEST}/a%20b%2Fc/forget`,
      `POST /issue_queries/${MANIFEST}/install_tool`,
    ]);
  });

  it("are refused in the tracker's own words where it will not run the query", async () => {
    const fleet = await serving(422, { code: "fleet.issue_query_refused", message: "Field 'colour' does not exist", run_id: "r", fields: {}, chain: [] });
    const read = await new IssuesHost(() => fleet.port).searchIssues(MANIFEST, "colour:red");
    expect(read).toMatchObject({ ok: false, outcome: { ok: false, why: "refused", error: { code: "fleet.issue_query_refused", message: "Field 'colour' does not exist" } } });
  });

  it("are not asked where Fleet is not connected", async () => {
    expect(await new IssuesHost(() => null).writeIssueQuery(MANIFEST, "open bugs")).toEqual({ ok: false, outcome: { ok: false, why: "not_connected" } });
  });
});
