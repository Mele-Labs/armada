// Reported bugs on the Issues store over a fake source and over a fake Fleet: read once and again when Fleet is reached again,
// replaced whole by `bug_reports.changed`, kept and dropped through Fleet's two routes with a refusal said in Fleet's words,
// opened as read, and the brief a kept report makes. Nothing here touches `window.armada`.

import type { BugReport, BugReports } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";
import { expect, test, vi } from "vitest";

import type { BugReportsApi } from "../../../shared/api/bug-reports";
import type { IssuesSource, ReportedBug } from "./model";
import { briefOf, jobOfReport, reportOfJob, reportsShown } from "./reports";
import { issuesSourceOf, reportedBugOf } from "./source";
import { createIssuesStore } from "./store";

const report = (id: string, over: Partial<ReportedBug> = {}): ReportedBug => ({
  id,
  session_id: "job-1",
  job_title: "Stop watches when their pull request merges",
  title: `Report ${id}`,
  what: "A watch stayed live.",
  steps: ["Start a watch.", "Merge the pull request."],
  errors: ["fleet.watch_live: a live watch holds this Job open"],
  logs: ["fleet.log"],
  at: "2026-10-29T10:00:00Z",
  kept: false,
  ...over,
});

const wire = (one: ReportedBug): BugReport => {
  const { kept, ...rest } = one;
  return { ...rest, ...(kept ? { kept: one.at } : {}) };
};

const refusal = { ok: false, outcome: { ok: false, why: "refused", error: { code: "fleet.no_such_bug_report", message: "There is no such bug report", run_id: "r", fields: {}, chain: [] } } } as never;

/** A Fleet holding `held`, answering the three routes and telling every follower the list whole after a write. */
function fleet(held: ReportedBug[]) {
  const listeners = new Set<(reports: BugReports) => void>();
  const calls = { list: 0, keep: [] as string[], drop: [] as string[] };
  const tell = () => listeners.forEach((on) => on({ reports: held.map(wire) }));
  const api: BugReportsApi = {
    listBugReports: async () => (calls.list++, { ok: true, value: { reports: held.map(wire) } }),
    keepBugReport: async (id) => {
      calls.keep.push(id);
      const one = held.find((it) => it.id === id);
      if (one === undefined) return refusal;
      held = held.map((it) => (it.id === id ? { ...it, kept: true } : it));
      return { ok: true, value: wire(held.find((it) => it.id === id)!) };
    },
    dropBugReport: async (id) => {
      calls.drop.push(id);
      if (!held.some((it) => it.id === id)) return refusal;
      held = held.filter((it) => it.id !== id);
      return { ok: true, value: { id } };
    },
    onBugReportsChanged: (on) => (listeners.add(on), () => void listeners.delete(on)),
  };
  return { api, calls, tell, set: (next: ReportedBug[]) => (held = next) };
}

const store = (api: BugReportsApi) => createIssuesStore({ ...issuesSourceOf({ getIssue: async () => refusal } as never, api) } as IssuesSource);
const settled = () => new Promise((resume) => setTimeout(resume, 0));

test("a report is the wire's with kept read as whether Fleet has a time for it", () => {
  const kept = reportedBugOf({ ...wire(report("a")), kept: "2026-10-29T11:00:00Z" });
  expect(kept.kept).toBe(true);
  expect(kept).not.toHaveProperty("kept", "2026-10-29T11:00:00Z");
  expect(reportedBugOf(wire(report("b"))).kept).toBe(false);
});

test("the reports are read from list_bug_reports once, however many surfaces ask", async () => {
  const f = fleet([report("a"), report("b")]);
  const issues = store(f.api);
  issues.readReports();
  issues.readReports();
  await settled();
  expect(f.calls.list).toBe(1);
  expect(issues.get().reports.map((one) => one.id)).toEqual(["a", "b"]);
});

test("a read Fleet refused is asked again by the next surface, and again when Fleet is reached", async () => {
  const f = fleet([report("a")]);
  const failing = vi.spyOn(f.api, "listBugReports").mockResolvedValueOnce({ ok: false, outcome: { ok: false, why: "not_connected" } });
  const issues = store(f.api);
  issues.readReports();
  await settled();
  expect(issues.get().reports).toEqual([]);
  issues.readReports();
  await settled();
  expect(failing).toHaveBeenCalledTimes(2);
  expect(issues.get().reports.map((one) => one.id)).toEqual(["a"]);
  f.set([report("a"), report("c")]);
  issues.loadReports();
  await settled();
  expect(issues.get().reports.map((one) => one.id)).toEqual(["a", "c"]);
});

test("bug_reports.changed replaces the list whole, and stops being heard once unfollowed", async () => {
  const f = fleet([report("a")]);
  const issues = store(f.api);
  const stop = issues.followReports();
  f.set([report("b"), report("a", { kept: true })]);
  f.tell();
  expect(issues.get().reports.map((one) => [one.id, one.kept])).toEqual([["b", false], ["a", true]]);
  stop();
  f.set([]);
  f.tell();
  expect(issues.get().reports).toHaveLength(2);
});

test("a report open in the reader is put away when the list no longer holds it", async () => {
  const f = fleet([report("a"), report("b")]);
  const issues = store(f.api);
  issues.readReports();
  await settled();
  issues.followReports();
  issues.openReport("a");
  f.set([report("b")]);
  f.tell();
  expect(issues.get().openReport).toBeUndefined();
});

test("opening a report marks it read, once, and puts any issue away", async () => {
  const f = fleet([report("a")]);
  const issues = store(f.api);
  issues.open("armada", "#1");
  issues.openReport("a");
  issues.openReport("a");
  expect(issues.get().seen).toEqual(["a"]);
  expect(issues.get().open).toBeUndefined();
  expect(issues.get().openReport).toBe("a");
});

test("Keep calls keep_bug_report and shows the report kept once Fleet answers", async () => {
  const f = fleet([report("a"), report("b")]);
  const issues = store(f.api);
  issues.readReports();
  await settled();
  await issues.keepReport("a");
  expect(f.calls.keep).toEqual(["a"]);
  expect(issues.get().reports.map((one) => [one.id, one.kept])).toEqual([["a", true], ["b", false]]);
  expect(issues.get().seen).toContain("a");
  expect(issues.get().failed).toBeUndefined();
});

test("Drop calls drop_bug_report, puts the report away and closes the reader on it", async () => {
  const f = fleet([report("a"), report("b")]);
  const issues = store(f.api);
  issues.readReports();
  await settled();
  issues.openReport("a");
  await issues.dropReport("a");
  expect(f.calls.drop).toEqual(["a"]);
  expect(issues.get().reports.map((one) => one.id)).toEqual(["b"]);
  expect(issues.get().openReport).toBeUndefined();
});

test("a drop of a report already gone is said in Fleet's words and the list is read again", async () => {
  const f = fleet([report("a")]);
  const issues = store(f.api);
  issues.readReports();
  await settled();
  f.set([]);
  await issues.dropReport("a");
  await settled();
  expect(issues.get().failed).toBe("There is no such bug report");
  expect(issues.get().reports).toEqual([]);
});

test("a keep Fleet refuses leaves the report as it was and says why", async () => {
  const f = fleet([report("a")]);
  const issues = store(f.api);
  issues.readReports();
  await settled();
  f.set([]);
  await issues.keepReport("a");
  expect(issues.get().failed).toBe("There is no such bug report");
});

test("the brief a kept report makes names the title, what went wrong, then each part the agent gave", () => {
  expect(briefOf(report("a", { title: "T", what: "W" }))).toBe(["T", "", "W", "", "Steps", "1. Start a watch.", "2. Merge the pull request.", "", "Errors", "fleet.watch_live: a live watch holds this Job open", "", "Logs", "fleet.log"].join("\n"));
  expect(briefOf(report("a", { title: "T", what: "W", steps: [], errors: [], logs: [] }))).toBe("T\n\nW");
});

test("a Job's mark opens its first unopened report, else its first", () => {
  const all = [report("a", { session_id: "x" }), report("b", { session_id: "y" }), report("c", { session_id: "y" })];
  expect(reportOfJob(all, [], "y")?.id).toBe("b");
  expect(reportOfJob(all, ["b"], "y")?.id).toBe("c");
  expect(reportOfJob(all, ["b", "c"], "y")?.id).toBe("b");
  expect(reportOfJob(all, [], "z")).toBeUndefined();
});

test("the band is drawn under All and under the armada repository, and under no other", () => {
  const served = (id: string) => ({ root: `/code/${id}`, records_root: `/records/${id}`, manifest: { id, repository: id, path: `${id}/armada.yml` } }) as never;
  const armada = served("armada");
  const storefront = served("storefront");
  expect(reportsShown(null, [armada, storefront])).toBe(true);
  expect(reportsShown(armada, [armada, storefront])).toBe(true);
  expect(reportsShown(storefront, [armada, storefront])).toBe(false);
  expect(reportsShown(null, [storefront])).toBe(false);
});

const job = (id: string, attachments: Session["attachments"]) => ({ id, attachments }) as unknown as Session;

test("a report is fixed when Fleet's report carries the merge that fixed it, and open when it does not", () => {
  const fixed = { at: "2026-10-30T10:00:00Z", pull_request: 3071, title: "Read one icon table", url: "https://github.com/acme/armada/pull/3071" };
  expect(reportedBugOf(wire(report("a", { fixed }))).fixed).toEqual(fixed);
  expect(reportedBugOf(wire(report("b"))).fixed).toBeUndefined();
});

test("the Job started on a report is the one holding its report row, and a Job that gave the row back is no one's", () => {
  const holding = job("holding", [{ kind: "report", id: "a", title: "Report a" }]);
  const other = job("other", [{ kind: "report", id: "b", title: "Report b" }]);
  const gaveBack = job("gave-back", []);
  expect(jobOfReport([gaveBack, other, holding], "a")?.id).toBe("holding");
  expect(jobOfReport([gaveBack, other], "a")).toBeUndefined();
});
