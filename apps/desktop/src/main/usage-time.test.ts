// Where Jobs' time went is three requests, and the only thing that crosses from the renderer is a Job's id.

import { expect, test } from "vitest";

import { sessionOf } from "./session-system-channels";
import { USAGE_HOURS_PATH, USAGE_TIME_PATH, UsageTimeHost, jobTimePath } from "./usage-time";

test("the window is Fleet's own, so the two page reads ask for no bound", () => {
  expect(USAGE_TIME_PATH).toBe("/usage/time");
  expect(USAGE_HOURS_PATH).toBe("/usage/hours");
});

test("a Job's path carries its id, encoded, and nothing else", () => {
  expect(jobTimePath("200c15cc-2a83-4c4e-982d-710f7d182eb5")).toBe(
    "/sessions/time?session_id=200c15cc-2a83-4c4e-982d-710f7d182eb5",
  );
  expect(jobTimePath("a&b=c d")).toBe("/sessions/time?session_id=a%26b%3Dc%20d");
});

test("only a non-empty string is passed on as a Job's id", () => {
  expect(sessionOf("job-1")).toBe("job-1");
  for (const bad of [
    "",
    42,
    null,
    undefined,
    {},
    [
      "job-1",
    ],
  ])
    expect(sessionOf(bad)).toBeNull();
});

test("with no port the three reads say so and ask nothing", async () => {
  const host = new UsageTimeHost(() => null);
  const unsent = {
    ok: false,
    outcome: {
      ok: false,
      why: "not_connected",
    },
  };
  expect(await host.time()).toEqual(unsent);
  expect(await host.hours()).toEqual(unsent);
  expect(await host.job("job-1")).toEqual(unsent);
});
