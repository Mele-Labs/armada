// The Cockpit and the Dashboard on a real Fleet: no draft, and the Now of every active Job built from
// what main holds for it. In `mock/` because it loads the Jobs surface, which `desktop` does not.

import { afterEach, describe, expect, test, vi } from "vitest";

import type { CallView } from "@armada/jobs/draft/calls";
import type { NowView } from "@armada/jobs/draft/now";
import type { Active } from "@armada/protocol";
import { sampleDetail, sampleJob, sampleStep } from "@armada/screens/src/draft/sample";

import { answerJobAsk } from "../cockpit/answer-job";
import { asksAnAgent } from "../cockpit/standing";
import { feedOf } from "../real-feed";

const DRONE = "01M3WJ6FGZ003DCX123T7W6YP1";
const JOB = "01M130Y1380016YK5S0JXBXDQ5";
const NOW = Date.parse("2026-10-12T10:00:00Z");

const running = sampleJob({ id: JOB, current_step_id: "implement", title: "Cap the retry backoff" });

/** What main holds for a Job on its Implement step with one Drone at work, as the wire shapes it. */
function holding(over: Parameters<typeof sampleDetail>[0] = {}): Active {
  return {
    [JOB]: {
      detail: sampleDetail({ job: running, steps: [sampleStep({ step_id: "implement", label: "Implement" })], ...over }),
      drones: {
        job_id: JOB,
        drones: [
          {
            drone_id: DRONE,
            step_id: "implement",
            state: "running",
            since: "2026-10-12T09:50:00Z",
            activity: { line: "Edit retry.rs", tail: ["Read retry.rs", "Grep backoff", "Edit retry.rs"] },
          },
        ],
      },
    },
  };
}

afterEach(() => vi.unstubAllGlobals());

describe("the Now of an active Job on a real Fleet", () => {
  test("a running Job's Drone shows its line and tail, in its Now and on its call card", () => {
    const { nows, calls } = feedOf(holding(), [running], undefined, NOW);
    const drone = nows?.[JOB]?.running?.[0];
    expect(drone).toMatchObject({ of: "drone", name: "Implement Drone", line: "Edit retry.rs", tail: ["Read retry.rs", "Grep backoff", "Edit retry.rs"] });
    expect(calls?.[JOB]).toMatchObject({ served: true, running: [{ line: "Edit retry.rs" }] });
  });

  test("a Job whose Drone asked is a call that names the question it is answered through", () => {
    const asking = {
      question_id: "Q1",
      step_id: "implement",
      asked_at: "2026-10-12T09:55:00Z",
      question: "Which clock does the fixture pin?",
      options: [{ label: "Store", consequence: "Pins the store clock" }],
    };
    const { calls } = feedOf(holding({ asking }), [running], undefined, NOW);
    expect(calls?.[JOB]?.asks).toEqual([
      { key: "ask:Q1", kind: "drone", name: "Implement Drone", text: "Which clock does the fixture pin?", target: DRONE, asked: { kind: "question", id: "Q1" } },
    ]);
  });

  test("a Job that is over, or not read yet, draws nothing, and no Fleet at all is no feed", () => {
    expect(feedOf({}, [], undefined, NOW)).toEqual({ nows: undefined, calls: undefined });
    expect(feedOf({ [JOB]: {} }, [running], undefined, NOW).nows).toBeUndefined();
    const over = holding({ job: sampleJob({ id: JOB, status: "completed_success" }) });
    expect(feedOf(over, [running], undefined, NOW).nows).toBeUndefined();
  });

  test("the draft wins for a Job it names, and a Job it does not name is still read", () => {
    const other = "01OTHERJOB";
    const drafted: NowView = { running: [{ key: "drone:mock", of: "drone", name: "Mock Drone", state: "running", line: "from the draft" }] };
    const call: CallView = { request: "from the draft" };
    const active = { ...holding(), [other]: holding()[JOB]! };
    const { nows, calls } = feedOf(active, [running], { now: { [JOB]: drafted }, calls: { [JOB]: call } }, NOW);
    expect(nows?.[JOB]).toBe(drafted);
    expect(calls?.[JOB]).toBe(call);
    expect(nows?.[other]?.running?.[0]?.line).toBe("Edit retry.rs");
    expect(calls?.[other]?.served).toBe(true);
  });
});

describe("best and quick on a Job's ask", () => {
  function fleetAnswering() {
    const answerQuestion = vi.fn(async () => ({ ok: true as const }));
    const answerJudge = vi.fn(async () => ({ ok: true as const }));
    vi.stubGlobal("window", { armada: { answerQuestion, answerJudge } });
    return { answerQuestion, answerJudge };
  }

  test("on a Drone's question each sends the mode alone", async () => {
    const fleet = fleetAnswering();
    await answerJobAsk(JOB, { kind: "question", id: "Q1" }, "best")?.();
    await answerJobAsk(JOB, { kind: "question", id: "Q1" }, "quick")?.();
    expect(fleet.answerQuestion.mock.calls).toEqual([
      [JOB, "Q1", undefined, "best"],
      [JOB, "Q1", undefined, "quick"],
    ]);
  });

  test("on a Judge's question the refusal stands, and the mode says how the Drone goes about it", async () => {
    const fleet = fleetAnswering();
    await answerJobAsk(JOB, { kind: "judge", at: "2026-10-12T09:40:00Z" }, "quick")?.();
    expect(fleet.answerJudge.mock.calls).toEqual([[JOB, "2026-10-12T09:40:00Z", "agree", undefined, "quick"]]);
  });

  test("a command takes neither, and an ask the mock drew only clears", () => {
    expect(answerJobAsk(JOB, { kind: "command" }, "best")).toBeUndefined();
    expect(answerJobAsk(JOB, undefined, "best")).toBeUndefined();
    const command = { kind: "Drone question", key: `${JOB}:command:c1`, asked: { kind: "command" } } as unknown as Parameters<typeof asksAnAgent>[0];
    expect(asksAnAgent(command)).toBe(false);
  });

  test("a refusal comes back in Fleet's words", async () => {
    const refused = { ok: false as const, why: "refused" as const, error: { code: "x", message: "that question is not the one outstanding", run_id: "", fields: {}, chain: [] } };
    vi.stubGlobal("window", { armada: { answerQuestion: vi.fn(async () => refused) } });
    expect(await answerJobAsk(JOB, { kind: "question", id: "Q0" }, "best")?.()).toEqual({ kind: "refused", said: "that question is not the one outstanding" });
  });
});
