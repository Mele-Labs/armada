// A Job the Operator's Dispatch starts gets its brief as its first message. On 9 Nov one started empty and sat idle:
// `startSession`'s request only names a Job, and nothing sent the agent the words.

import { afterEach, expect, test, vi } from "vitest";

import { startOnBrief } from "./fleet";

const BRIEF = "Find the update loop behind React error #185 in the overview region.";

afterEach(() => {
  vi.unstubAllGlobals();
});

function fleet(started: { ok: boolean }) {
  const armada = {
    startSession: vi.fn(async () =>
      started.ok
        ? {
            ok: true,
            value: {
              id: "job-1",
            },
          }
        : {
            ok: false,
            outcome: {
              ok: false,
              why: "no_manifest",
            },
          },
    ),
    sendSessionMessage: vi.fn(async () => ({
      ok: true,
      value: {
        id: "job-1",
      },
    })),
  };
  vi.stubGlobal("armada", armada);
  return armada;
}

test("the brief goes to the started Job as its first message", async () => {
  const armada = fleet({
    ok: true,
  });

  const started = await startOnBrief("Find the update loop", "/repo", BRIEF);

  expect(started.ok).toBe(true);
  expect(armada.sendSessionMessage).toHaveBeenCalledWith({
    session_id: "job-1",
    text: BRIEF,
  });
});

test("a refused start sends nothing", async () => {
  const armada = fleet({
    ok: false,
  });

  const started = await startOnBrief("Find the update loop", "/repo", BRIEF);

  expect(started.ok).toBe(false);
  expect(armada.sendSessionMessage).not.toHaveBeenCalled();
});
