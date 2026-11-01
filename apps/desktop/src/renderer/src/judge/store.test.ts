// What a Job's `judge_readings` are to the components, and what the components' acts are to the Sessions' (`SessionJudgeActs`). No window: the store is read as it stands.

import { afterEach, expect, test, vi } from "vitest";
import type { JudgeReading } from "@armada/protocol";

import { judgeApiOver, provideJudge, putReadings, recordsOf } from "./store";

const ARTIFACT = { kind: "pull_request" as const, id: "2418", title: "#2418" };
const reading = (more: Partial<JudgeReading> = {}): JudgeReading => ({ id: "r1", artifact: ARTIFACT, prompt: "", state: "done", asked_at: "2026-10-31T14:00:00Z", read: false, notes: [], ...more });

afterEach(() => provideJudge(undefined));

test("a Job's readings are records of that Job, in the order Fleet kept them", () => {
  const first = reading({ id: "r1" });
  const second = reading({ id: "r2", artifact: { kind: "plan", id: "2", title: "Plan v2" } });
  expect(recordsOf("job-1", [first, second]).map((one) => [one.id, one.job, one.artifact.kind])).toEqual([
    ["r1", "job-1", "pull_request"],
    ["r2", "job-1", "plan"],
  ]);
  expect(recordsOf("job-1", undefined)).toEqual([]);
});

test("a note settled is how it was settled, and a note waiting has none", () => {
  const [one] = recordsOf("job-1", [
    reading({
      notes: [
        { id: "n1", claim: "Sent one", settled: { how: "sent", at: "2026-10-31T14:03:00Z" } },
        { id: "n2", claim: "Dismissed one", settled: { how: "dismissed", at: "2026-10-31T14:04:00Z" } },
        { id: "n3", claim: "Waiting one", where: "a.rs:1", about: "a.rs:1" },
      ],
    }),
  ]);
  expect(one!.notes.map((note) => note.settled)).toEqual(["sent", "dismissed", undefined]);
  expect(one!.notes[2]).toEqual({ id: "n3", claim: "Waiting one", where: "a.rs:1", about: "a.rs:1" });
});

test("a reading keeps the states Fleet gave it, a failed one with its words and no notes", () => {
  const [going, failed] = recordsOf("job-1", [reading({ state: "reading" }), reading({ id: "r2", state: "failed", failed: "The read stopped", finished_at: "2026-10-31T14:01:00Z" })]);
  expect(going!.state).toBe("reading");
  expect([failed!.state, failed!.failed, failed!.notes]).toEqual(["failed", "The read stopped", []]);
});

test("the readings of a Job replace the ones it had, and another Job's stay", () => {
  const acts = { ask: vi.fn(), send: vi.fn(), dismiss: vi.fn(), markRead: vi.fn() };
  const api = judgeApiOver(acts);
  putReadings("job-1", [reading({ id: "r1" })]);
  putReadings("job-2", [reading({ id: "r9" })]);
  putReadings("job-1", [reading({ id: "r2", artifact: { kind: "plan", id: "1", title: "Plan v1" } })]);
  for (const id of ["r1", "r2", "r9"]) api.markRead(id);
  // r1 was replaced, so nothing is asked for it; r2 and r9 reach their own Jobs.
  expect(acts.markRead.mock.calls).toEqual([["job-1", "r2"], ["job-2", "r9"]]);
});

test("the acts look up the Job from the reading they name", () => {
  const acts = { ask: vi.fn(), send: vi.fn(), dismiss: vi.fn(), markRead: vi.fn() };
  const api = judgeApiOver(acts);
  putReadings("job-7", [reading({ id: "r7" })]);
  api.send("r7", "n1");
  api.dismiss("r7", "n2");
  api.markRead("r7");
  api.ask({ job: "job-7", artifact: ARTIFACT, prompt: "tokens" });
  expect(acts.send).toHaveBeenCalledWith("job-7", "r7", "n1");
  expect(acts.dismiss).toHaveBeenCalledWith("job-7", "r7", "n2");
  expect(acts.markRead).toHaveBeenCalledWith("job-7", "r7");
  expect(acts.ask).toHaveBeenCalledWith("job-7", ARTIFACT, "tokens");
});

test("a reading no longer held is no act", () => {
  const acts = { ask: vi.fn(), send: vi.fn(), dismiss: vi.fn(), markRead: vi.fn() };
  judgeApiOver(acts).send("gone", "n1");
  expect(acts.send).not.toHaveBeenCalled();
});
