// What crosses from the renderer to run a shell line is checked before Fleet is asked, and the wait outlasts Fleet's own bound.

import { expect, test } from "vitest";

import { shellOf } from "./flight-controls-channels";
import { SHELL_MS } from "./flight-controls";

test("a Job id and a line are taken, and anything else is not", () => {
  expect(shellOf("s1", "pnpm typecheck")).toEqual({ sessionId: "s1", command: "pnpm typecheck" });
  expect(shellOf("", "ls")).toBeNull();
  expect(shellOf("s1", 3)).toBeNull();
  expect(shellOf(undefined, "ls")).toBeNull();
});

test("Bridge waits longer than Fleet's 120 seconds, so Fleet's timed_out is what arrives", () => {
  expect(SHELL_MS).toBeGreaterThan(120_000);
});
