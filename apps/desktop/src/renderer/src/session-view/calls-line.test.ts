// The line a run of tool calls reads as (`callsLine`): tense, how many places, and which call speaks for the run. Pure, so it is read here and not through a rendering.

import { expect, test } from "vitest";

import { callsLine } from "@armada/components/src/compositions/SessionThread/calls-line";

const calls = (...texts: string[]) => texts.map((text) => ({ text }));

test("a run in progress reads in the present tense and a finished one in the past", () => {
  const run = calls("Edit crates/store/src/audit.rs");
  expect(callsLine(run, true)).toBe("Editing audit.rs");
  expect(callsLine(run, false)).toBe("Edited audit.rs");
  expect(callsLine(calls("Read docs/scope.md"), true)).toBe("Reading scope.md");
  expect(callsLine(calls("Read docs/scope.md"), false)).toBe("Read scope.md");
});

test("one file is named and several are multiple files, with no count", () => {
  expect(callsLine(calls("Edit a/one.rs", "Edit b/one.rs"), true)).toBe("Editing one.rs");
  expect(callsLine(calls("Edit a/one.rs", "Write a/two.rs", "Edit a/three.rs"), true)).toBe("Editing multiple files");
});

test("several checks are counted, and other commands are not", () => {
  expect(callsLine(calls("Bash cargo check -p fleet", "Bash pnpm exec vitest run a.test.ts", "Bash armada check test x"), false)).toBe("Ran 3 checks");
  expect(callsLine(calls("Bash git status", "Bash ls"), true)).toBe("Running multiple commands");
  expect(callsLine(calls("Bash cd apps/desktop && cargo check"), true)).toBe("Running cargo check");
});

test("a tool Bridge does not know by name reads as its name", () => {
  expect(callsLine(calls("mcp__armada-fleet__list_jobs"), true)).toBe("list_jobs");
  expect(callsLine(calls("Mystery with some input"), false)).toBe("Mystery");
});

test("a bookkeeping call does not take the line from the work before it", () => {
  expect(callsLine(calls("Edit crates/store/src/audit.rs", "TaskUpdate 3 in_progress"), true)).toBe("Editing audit.rs");
  expect(callsLine(calls("Bash cargo test", "TaskCreate write the test"), true)).toBe("Running cargo test");
});
