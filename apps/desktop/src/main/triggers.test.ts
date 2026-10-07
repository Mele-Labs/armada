// A fired Trigger as Fleet sends it, read by Bridge. The string is the one
// `crates/ipc/src/tests/triggers.rs` asserts Fleet encodes, so a field renamed on either side
// fails in one of the two.

import { createServer, type Server } from "node:http";
import type { AddressInfo } from "node:net";

import { afterEach, expect, it } from "vitest";
import type { Event, JobTrigger } from "@armada/protocol";

import { Picked } from "./picked";
import { TriggerCommands } from "./triggers";

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

/** The string `crates/ipc/src/tests/triggers.rs` asserts Fleet encodes for a fix held for the owner. */
const FIX_READY =
  '{"name":"tidy","when":"step_passes","step":"implement","level":"machine","state":"fix_ready","exit_code":1,"started_at":"2026-10-07T10:00:00.000Z","log_at":"2026-10-07T10:00:02.000Z","repair":{"attempt":2,"branch":"armada/repair-tidy-1","files":["src/lib.rs","Cargo.toml"]}}';

it("reads a held fix the way Fleet writes it", () => {
  const trigger: JobTrigger = JSON.parse(FIX_READY);
  expect(trigger.state).toBe("fix_ready");
  expect(trigger.repair?.attempt).toBe(2);
  expect(trigger.repair?.files).toEqual(["src/lib.rs", "Cargo.toml"]);
  expect(trigger.repair?.choice).toBeUndefined();
  expect(trigger.ended_at).toBeUndefined();
});

it("reads a placed fix with the number of the pull request it opened", () => {
  const trigger: JobTrigger = JSON.parse(
    FIX_READY.replace('"state":"fix_ready"', '"state":"passed"').replace(
      '"Cargo.toml"]',
      '"Cargo.toml"],"choice":"new_pr","pull_request":{"url":"https://forge.test/o/r/pull/412","number":412}',
    ),
  );
  expect(trigger.repair?.choice).toBe("new_pr");
  expect(trigger.repair?.pull_request?.number).toBe(412);
});

let listening: Server | null = null;
afterEach(async () => {
  const server = listening;
  listening = null;
  if (server !== null) await new Promise<void>((done) => server.close(() => done()));
});

async function fleetAnswering(status: number, body: unknown, into: { method: string; path: string; body: string }[]): Promise<number> {
  const server = createServer((request, response) => {
    let text = "";
    request.on("data", (chunk) => (text += String(chunk)));
    request.on("end", () => {
      into.push({ method: request.method ?? "", path: request.url ?? "", body: text });
      response.writeHead(status, { "content-type": "application/json" });
      response.end(JSON.stringify(body));
    });
  });
  listening = server;
  await new Promise<void>((up) => server.listen(0, "127.0.0.1", up));
  return (server.address() as AddressInfo).port;
}

it("sends where a fix goes to the Job's own route, and is told Fleet took it", async () => {
  const asked: { method: string; path: string; body: string }[] = [];
  const port = await fleetAnswering(200, { state: "passed" }, asked);
  const answer = await new TriggerCommands(() => port, new Picked()).chooseFix("01JOB", { trigger: "tidy", choice: "new_pr" });
  expect(answer).toEqual({ ok: true });
  expect(asked).toEqual([{ method: "POST", path: "/jobs/01JOB/choose_trigger_fix", body: '{"trigger":"tidy","choice":"new_pr"}' }]);
});

it("hands a refusal back in Fleet's own words, so the branch can ask again", async () => {
  const asked: { method: string; path: string; body: string }[] = [];
  const port = await fleetAnswering(
    409,
    { code: "fleet.fix_conflicts", message: "the Job's branch moved and the fix conflicts in src/lib.rs, so nothing was merged", run_id: "r", fields: {}, chain: [] },
    asked,
  );
  const answer = await new TriggerCommands(() => port, new Picked()).chooseFix("01JOB", { trigger: "tidy", choice: "this_branch" });
  expect(answer.ok).toBe(false);
  expect(JSON.stringify(answer)).toContain("fleet.fix_conflicts");
  expect(await new TriggerCommands(() => null, new Picked()).chooseFix("01JOB", { trigger: "tidy", choice: "new_pr" })).toEqual({
    ok: false,
    why: "not_connected",
  });
});
