// What Fleet sees the Job doing (`detectedOf`): the calls since the last thing said, while the turn runs, and nothing otherwise.

import { expect, test } from "vitest";
import type { Session } from "@armada/screens/src/draft/sessions";

import { detectedOf } from "./detected";

const session = (rows: unknown[], over: Record<string, unknown> = {}): Session => ({ id: "s1", rows, turn: { state: "working" }, attachments: [], ...over }) as unknown as Session;
const tool = (text: string) => ({ kind: "tool", text });
const said = (text: string) => ({ kind: "message", text });

test("the calls at the end of the thread are the line, in the present tense", () => {
  expect(detectedOf(session([said("Looking at it"), tool("Read docs/scope.md"), tool("Edit crates/store/src/audit.rs")]))).toBe("Editing audit.rs");
});

test("calls before the last message are not the line", () => {
  expect(detectedOf(session([tool("Edit a.rs"), said("Done with that"), tool("Bash cargo test")]))).toBe("Running cargo test");
  expect(detectedOf(session([tool("Edit a.rs"), said("Done with that")]))).toBeUndefined();
});

test("nothing is detected unless the turn is working", () => {
  expect(detectedOf(session([tool("Edit a.rs")], { turn: { state: "idle" } }))).toBeUndefined();
  expect(detectedOf(session([tool("Edit a.rs")], { dead: { reason: "gone" } }))).toBeUndefined();
});
