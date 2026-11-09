// A kind's key: the tab of that kind comes to the front of its pane, or one opens where the Job has none, and never a second.

import { beforeEach, expect, test } from "vitest";

import { focusBesideThread, focusKind } from "./focus";
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

const plans = () =>
  panesOf(peek(JOB).root)
    .flatMap((one) => one.tabs)
    .filter((one) => one.kind === "plan");

test("a view with no tab opens as a tab of the thread's pane, in front", () => {
  expect(holderOf(peek(JOB), "plan")).toBeUndefined();
  focusBesideThread(JOB, "plan");
  const thread = holderOf(peek(JOB), "thread")!;
  expect(plans()).toHaveLength(1);
  expect(holderOf(peek(JOB), "plan")!.id).toBe(thread.id);
  expect(thread.tabs.find((one) => one.id === thread.active)?.kind).toBe("plan");
  expect(peek(JOB).focus).toBe(thread.id);
});

test("a view whose tab is open elsewhere is focused where it is, and no second opens", () => {
  const thread = holderOf(peek(JOB), "thread")!;
  // The Plan tab sits in the Now pane, behind a Changes tab that is its pane's front.
  const now = holderOf(peek(JOB), "now")!;
  rearrange(JOB, (arrangement) => addTab(arrangement, now.id, "plan"));
  rearrange(JOB, (arrangement) => addTab(arrangement, now.id, "changes"));
  const [plan] = plans();
  expect(holderOf(peek(JOB), "plan")!.id).toBe(now.id);
  expect(holderOf(peek(JOB), "plan")!.active).not.toBe(plan!.id);

  focusBesideThread(JOB, "plan");
  expect(plans()).toHaveLength(1);
  expect(holderOf(peek(JOB), "plan")!.id).toBe(now.id);
  expect(holderOf(peek(JOB), "plan")!.active).toBe(plan!.id);
  expect(peek(JOB).focus).toBe(now.id);
  expect(holderOf(peek(JOB), "thread")!.tabs).toEqual(thread.tabs);
});
