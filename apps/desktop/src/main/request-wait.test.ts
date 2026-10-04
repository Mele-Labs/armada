// How long `ask` waits for Fleet, and who decides it.
//
// undici's own bound is 300 s, so it is scaled down here: a dispatcher with a
// short `headersTimeout` stands in for the default, and an answer that takes
// longer than it stands in for the `rerun_checks` of 4 Oct 2026.

import { createServer, type Server } from "node:http";
import { once } from "node:events";
import type { AddressInfo } from "node:net";

import { Agent } from "undici";
import { afterEach, describe, expect, it } from "vitest";

import { ask, NO_WAIT } from "./request";
import { HOST } from "./runtime-file";

/** undici's 300 000 ms default, at this file's scale. */
const DEFAULT_SCALED_MS = 300;
/** How long Fleet takes to answer: past the scaled default, and past the
 *  second of slack undici's coarse header timer can add to it. */
const ANSWER_MS = 2_000;

const opened: (() => void)[] = [];

afterEach(() => {
  while (opened.length > 0) opened.pop()?.();
});

/** A Fleet that sends its headers `ANSWER_MS` after the request arrives. */
async function slowFleet(): Promise<number> {
  const server: Server = createServer((_request, response) => {
    const answering = setTimeout(() => {
      response.writeHead(200, { "content-type": "application/json" });
      response.end(JSON.stringify({ id: "job", status: "running" }));
    }, ANSWER_MS);
    response.on("close", () => clearTimeout(answering));
  });
  server.listen(0, HOST);
  await once(server, "listening");
  opened.push(() => server.closeAllConnections());
  opened.push(() => server.close());
  return (server.address() as AddressInfo).port;
}

describe("waiting on a slow Fleet", () => {
  it("undici's default headers timeout cuts a slow answer short of Bridge's wait", async () => {
    const port = await slowFleet();
    const scaledDefault = new Agent({ headersTimeout: DEFAULT_SCALED_MS });
    opened.push(() => void scaledDefault.close());

    const answer = await ask(port, "POST", "/jobs/job/rerun_checks", undefined, 10_000, scaledDefault);

    // The incident's own shape: unreachable, `fetch failed`, inside a wait that had not run out.
    expect(answer).toMatchObject({
      ok: false,
      outcome: { why: "transport", detail: "fetch failed", fault: { why: "unreachable" } },
    });
  });

  it("waits for Fleet's answer as long as Bridge's own wait allows", async () => {
    const port = await slowFleet();

    const answer = await ask(port, "POST", "/jobs/job/rerun_checks", undefined, 10_000);

    expect(answer).toEqual({ ok: true, body: { id: "job", status: "running" } });
  });

  it("waits with no bound at all at NO_WAIT", async () => {
    const port = await slowFleet();

    const answer = await ask(port, "POST", "/jobs/job/rerun_checks", undefined, NO_WAIT);

    expect(answer.ok).toBe(true);
  });

  it("still gives up at Bridge's own wait", async () => {
    const port = await slowFleet();

    const answer = await ask(port, "POST", "/jobs/job/rerun_checks", undefined, DEFAULT_SCALED_MS);

    expect(answer).toMatchObject({
      ok: false,
      outcome: { why: "transport", fault: { why: "timed_out", waitedMs: DEFAULT_SCALED_MS } },
    });
  });
});
