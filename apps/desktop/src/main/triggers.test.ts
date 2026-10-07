// A fired Trigger as Fleet sends it, read by Bridge. The string is the one
// `crates/ipc/src/tests/triggers.rs` asserts Fleet encodes, so a field renamed on either side
// fails in one of the two.

import { expect, it } from "vitest";
import type { Event, JobTrigger } from "@armada/protocol";

const FIRED =
  '{"name":"tidy","when":"step_passes","step":"implement","level":"machine","state":"failed","exit_code":1,"started_at":"2026-10-07T10:00:00.000Z","ended_at":"2026-10-07T10:00:02.000Z","log_at":"2026-10-07T10:00:02.000Z"}';

it("reads a fired Trigger the way Fleet writes it", () => {
  const trigger: JobTrigger = JSON.parse(FIRED);
  expect(trigger.state).toBe("failed");
  expect(trigger.exit_code).toBe(1);
  expect(trigger.when).toBe("step_passes");
  expect(trigger.log_at).toBe(trigger.ended_at);
});

it("tells the event apart by its kind and carries the row whole", () => {
  const event: Event = JSON.parse(
    `{"kind":"job.trigger_changed","job_id":"01JOB","trigger":${FIRED},"at":"2026-10-07T10:00:02.000Z"}`,
  );
  if (event.kind !== "job.trigger_changed") throw new Error("the kind narrows the event");
  expect(event.trigger.name).toBe("tidy");
});
