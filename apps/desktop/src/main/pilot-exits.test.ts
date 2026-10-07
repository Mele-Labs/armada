// The three ways out of a pilot, read off a real listener: the route each goes to, the body it
// carries, and what is done with the answer. **What crosses the wire is under test**, so a route
// named wrongly or a note dropped fails here rather than on a person's press.

import { createServer, type Server } from "node:http";
import type { AddressInfo } from "node:net";

import { afterEach, expect, it } from "vitest";

import type { JobSummary } from "@armada/protocol";
import { job } from "@armada/screens/src/fixtures/build/base";
import { PilotExits } from "./pilot-exits";

let listening: Server | null = null;

afterEach(async () => {
  const server = listening;
  listening = null;
  if (server === null) return;
  await new Promise<void>((done) => server.close(() => done()));
});

type Sent = { method: string; path: string; body: unknown };

/** A Fleet that answers every exit with `answer`, at `status`. */
async function fleet(sent: Sent[], status: number, answer: unknown): Promise<number> {
  const server = createServer((request, response) => {
    let raw = "";
    request.on("data", (chunk) => (raw += String(chunk)));
    request.on("end", () => {
      sent.push({ method: request.method ?? "", path: request.url ?? "", body: raw === "" ? undefined : JSON.parse(raw) });
      response.writeHead(status, { "content-type": "application/json" });
      response.end(JSON.stringify(answer));
    });
  });
  listening = server;
  await new Promise<void>((done) => server.listen(0, "127.0.0.1", () => done()));
  return (server.address() as AddressInfo).port;
}

function exits(port: number | null) {
  const folded: JobSummary[] = [];
  const refreshed: string[] = [];
  const read: number[] = [];
  const pilot = new PilotExits({
    port: () => port,
    fold: (row) => folded.push(row),
    reread: async (at) => void read.push(at),
    refresh: (_port, jobId) => void refreshed.push(jobId),
  });
  return { pilot, folded, refreshed, read };
}

const piloted = (exit: string): JobSummary => job("completed_success", { id: "J1", piloted: { reason: "take_over", since: "2026-10-07T13:00:00Z", exit, ended_at: "2026-10-07T13:30:00Z" } });

it("sends each exit to its own route under the Job, with a person's note where they gave one", async () => {
  const sent: Sent[] = [];
  const port = await fleet(sent, 200, piloted("attested"));
  const { pilot } = exits(port);
  await pilot.exit("J1", "submit");
  await pilot.exit("J1", "attest", "landed by hand");
  await pilot.exit("J1", "supersede");
  expect(sent.map((one) => `${one.method} ${one.path}`)).toEqual([
    "POST /jobs/J1/submit_for_verification",
    "POST /jobs/J1/attest_complete",
    "POST /jobs/J1/close_as_superseded",
  ]);
  expect(sent[0]!.body).toBeUndefined();
  expect(sent[1]!.body).toEqual({ note: "landed by hand" });
  expect(sent[2]!.body).toEqual({});
});

it("folds the Job Fleet answered with onto the Board, and reads the Job again", async () => {
  const sent: Sent[] = [];
  const port = await fleet(sent, 200, piloted("attested"));
  const { pilot, folded, refreshed } = exits(port);
  expect(await pilot.exit("J1", "attest")).toEqual({ ok: true });
  expect(folded).toMatchObject([{ id: "J1", piloted: { exit: "attested" } }]);
  expect(refreshed).toEqual(["J1"]);
});

it("reads the whole Board again where the answer was not a row", async () => {
  const sent: Sent[] = [];
  const port = await fleet(sent, 200, { not: "a job" });
  const { pilot, folded, read } = exits(port);
  await pilot.exit("J1", "supersede");
  expect(folded).toEqual([]);
  expect(read).toEqual([port]);
});

it("hands back Fleet's refusal as it came, and folds nothing", async () => {
  const sent: Sent[] = [];
  const port = await fleet(sent, 409, { code: "fleet.steps_not_advanced", message: "A step has not advanced." });
  const { pilot, folded } = exits(port);
  expect(await pilot.exit("J1", "attest")).toMatchObject({ ok: false, why: "refused", error: { code: "fleet.steps_not_advanced", message: "A step has not advanced." } });
  expect(folded).toEqual([]);
});

it("refuses a second press of one Job's exit while the first is out, and sends nothing without Fleet", async () => {
  const sent: Sent[] = [];
  const port = await fleet(sent, 200, piloted("submitted"));
  const { pilot } = exits(port);
  const [first, second] = await Promise.all([pilot.exit("J1", "submit"), pilot.exit("J1", "submit")]);
  expect([first, second]).toContainEqual({ ok: false, why: "already_piloting" });
  expect(sent).toHaveLength(1);
  expect(await exits(null).pilot.exit("J1", "attest")).toEqual({ ok: false, why: "not_connected" });
});
