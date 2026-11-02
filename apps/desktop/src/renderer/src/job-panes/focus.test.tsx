// A kind's key: the tab of that kind comes to the front of its pane, or one opens where the Job has none, and never a second.

import { beforeEach, expect, test } from "vitest";

import { focusKind } from "./focus";
import { forgetPanes, jobOpens, peek, rearrange } from "./store";
import { addTab, holderOf, panesOf } from "./tree";

const JOB = "focus-job";
const terminals = () =>
  panesOf(peek(JOB).root)
    .flatMap((one) => one.tabs)
    .filter((one) => one.kind === "terminal");

beforeEach(() => forgetPanes());

test("a Job with no Terminal tab gets one, and it is the pane's front tab", () => {
  expect(holderOf(peek(JOB), "terminal")).toBeUndefined();
  focusKind(JOB, "terminal");
  const holder = holderOf(peek(JOB), "terminal")!;
  expect(terminals()).toHaveLength(1);
  expect(holder.tabs.find((one) => one.id === holder.active)?.kind).toBe("terminal");
});

test("a Terminal tab behind another comes to the front, and the key opens no second", () => {
  jobOpens(JOB, "terminal");
  const [terminal] = terminals();
  // A Files tab opens in front of it, in the same pane.
  const pane = holderOf(peek(JOB), "terminal")!;
  rearrange(JOB, (arrangement) => addTab(arrangement, pane.id, "files"));
  expect(holderOf(peek(JOB), "terminal")!.active).not.toBe(terminal!.id);

  focusKind(JOB, "terminal");
  expect(holderOf(peek(JOB), "terminal")!.active).toBe(terminal!.id);
  expect(terminals()).toHaveLength(1);
});
