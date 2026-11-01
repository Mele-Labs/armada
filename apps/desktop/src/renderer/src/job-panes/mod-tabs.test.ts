// A tab mod a Job builds opens in that Job's panes, bound to the mod, once; one the + → Custom composer started is bound already and gets no second.

import { expect, test } from "vitest";
import type { ModSummary } from "@armada/protocol";

import { openBuiltTabs } from "./mod-tabs";
import { jobOpens, peek } from "./store";
import { panesOf } from "./tree";

const mod = (name: string, made_by: string, kind: ModSummary["kind"] = "tab"): ModSummary => ({ name, kind, made_by, enabled: true, valid: true, changed_at: "t1" }) as ModSummary;
const customs = (jobId: string) => panesOf(peek(jobId).root).flatMap((one) => one.tabs).filter((one) => one.kind === "custom");

test("a tab mod the open Job made lands as a Custom tab bound to it, once", () => {
  const JOB = "builder";
  openBuiltTabs(JOB, [mod("track-view", JOB)]);
  expect(customs(JOB)).toEqual([expect.objectContaining({ kind: "custom", mod: "track-view", label: "Track view" })]);
  openBuiltTabs(JOB, [mod("track-view", JOB)]);
  expect(customs(JOB)).toHaveLength(1);
});

test("a second tab mod of the Job joins the pane holding the first", () => {
  const JOB = "builder-two";
  openBuiltTabs(JOB, [mod("one", JOB)]);
  openBuiltTabs(JOB, [mod("one", JOB), mod("two", JOB)]);
  const holders = panesOf(peek(JOB).root).filter((pane) => pane.tabs.some((tab) => tab.kind === "custom"));
  expect(holders).toHaveLength(1);
  expect(customs(JOB).map((tab) => tab.mod)).toEqual(["one", "two"]);
});

test("a tab already bound to the mod gets no second copy", () => {
  const JOB = "composer-job";
  jobOpens(JOB, "custom", "Track view", undefined, "track-view");
  openBuiltTabs(JOB, [mod("track-view", JOB)]);
  expect(customs(JOB)).toHaveLength(1);
});

test("a mod of another kind or another Job opens nothing", () => {
  const JOB = "quiet-job";
  openBuiltTabs(JOB, [mod("dark", JOB, "theme"), mod("elsewhere", "other-job")]);
  expect(customs(JOB)).toHaveLength(0);
});
