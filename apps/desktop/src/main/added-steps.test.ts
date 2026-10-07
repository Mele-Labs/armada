// A step added to a Job as Fleet sends it, read by Bridge. The strings are the ones
// `crates/ipc/src/tests/added_steps.rs` asserts Fleet encodes, so a field renamed on either side
// fails in one of the two.

import { expect, it } from "vitest";
import type { AddedStep, AddStep, Event, SaveTrigger } from "@armada/protocol";

const ADDED =
  '{"id":"a1","runs":{"kind":"script","command":"fmt"},"when":"step_passes","step":"implement","block":false,"repair":true,"placed":"running","added_at":"2026-10-07T10:00:00.000Z","state":"passed","exit_code":0,"started_at":"2026-10-07T10:00:01.000Z","ended_at":"2026-10-07T10:00:02.000Z","log_at":"2026-10-07T10:00:02.000Z","kept":"machine"}';

it("reads an added step the way Fleet writes it", () => {
  const added: AddedStep = JSON.parse(ADDED);
  expect(added.runs).toEqual({ kind: "script", command: "fmt" });
  expect(added.state).toBe("passed");
  expect(added.kept).toBe("machine");
  expect(added.log_at).toBe(added.ended_at);
});

it("tells the event apart by its kind and carries the row whole", () => {
  const event: Event = JSON.parse(
    `{"kind":"job.addition_changed","job_id":"01JOB","addition":${ADDED},"at":"2026-10-07T10:00:02.000Z"}`,
  );
  if (event.kind !== "job.addition_changed") throw new Error("the kind narrows the event");
  expect(event.addition.id).toBe("a1");
  expect(event.removed).toBeUndefined();
});

it("sends a step to add and a save that keeps one as Fleet decodes them", () => {
  const add: AddStep = { runs: { kind: "skill", skill: "tidy-up" }, when: "pr_opened", step: "summarise", repair: true };
  expect(JSON.stringify(add)).toBe(
    '{"runs":{"kind":"skill","skill":"tidy-up"},"when":"pr_opened","step":"summarise","repair":true}',
  );
  const save: SaveTrigger = {
    scope: "machine",
    definition: "name: x\n",
    kept_from: { job_id: "01JOB", addition_id: "a1" },
  };
  expect(JSON.stringify(save)).toContain('"kept_from":{"job_id":"01JOB","addition_id":"a1"}');
});
