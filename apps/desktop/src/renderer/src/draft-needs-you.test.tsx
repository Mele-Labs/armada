// A draft a Job put to him is listed in its Needs you, so it is found when its card sits behind Show earlier in a long thread. Pressing the row opens the card as a tab
// of the Job, named for the draft; a proposed workflow is still approved from its facts, where it stands.

import { afterEach, beforeEach, expect, test, vi } from "vitest";
import type { Session } from "@armada/screens/src/draft/sessions";

import { labelOf } from "./job-panes/PaneView";
import { forgetPanes, look, peek } from "./job-panes/store";
import { activate, holderOf, panesOf } from "./job-panes/tree";
import { drawLayersAsTabs } from "./session-view/layers";
import { entriesOf } from "./sessions";

const AT = "2026-10-12T09:14:02.000Z";
const JOB = "draft-needs-you-job";
const TITLE = "Description for the queue pull request";
const goes = { onGoTo: vi.fn(), onOpenLink: vi.fn(), onOpenCheck: vi.fn(), onOpenLoadout: vi.fn() };

// Fleet's waiting item for a draft: `Approve` is the one option it offers.
const waiting = { id: "draft:d1", text: TITLE, since: AT, source: "draft", act: { kind: "answer", target: "draft:d1" }, options: [{ label: "Approve" }] };

const jobWith = (row: Record<string, unknown>): Session =>
  ({ id: JOB, attachments: [], turn: { state: "idle" }, rows: [{ id: "d1", at: AT, kind: "draft", title: TITLE, text: "Retries the lease once.", ...row }], waitingFor: [waiting] }) as unknown as Session;

function needsYou(session: Session) {
  const answerWaiting = vi.fn();
  const rows = entriesOf(session, goes, vi.fn(), vi.fn(), [session], vi.fn(), vi.fn(), answerWaiting, vi.fn(), { slots: [] }, undefined);
  return rows.find((one) => one.kind === "waiting");
}

const draftTabs = () => panesOf(peek(JOB).root).flatMap((pane) => pane.tabs).filter((tab) => tab.kind === "draft");

let release: (() => void) | undefined;
beforeEach(() => {
  forgetPanes();
  // The Job is drawn in panes, as the workspace has it.
  release = drawLayersAsTabs(JOB, () => false);
});
afterEach(() => release?.());

test("an open draft is listed in Needs you with no Approve, since its text is out of sight there", () => {
  const row = needsYou(jobWith({}));
  expect(row, "the draft is listed in Needs you").toBeDefined();
  expect(row!.text).toBe(TITLE);
  expect(row!.options).toBeUndefined();
});

test("pressing the draft's row opens a tab named for the draft beside the thread, and pressing again opens no second", () => {
  const row = needsYou(jobWith({}));
  expect(row, "the draft is listed in Needs you").toBeDefined();
  row!.onOpen();
  expect(draftTabs().map(labelOf)).toEqual([TITLE]);
  const pane = holderOf(peek(JOB), "draft")!;
  expect(pane.tabs.map((tab) => tab.kind)).toContain("thread");
  expect(pane.tabs.find((tab) => tab.id === pane.active)?.kind).toBe("draft");

  // The person goes back to the thread, then presses the row again: the same tab comes forward.
  const thread = pane.tabs.find((tab) => tab.kind === "thread")!;
  look(JOB, (arrangement) => activate(arrangement, thread.id));
  row!.onOpen();
  expect(draftTabs()).toHaveLength(1);
  const after = holderOf(peek(JOB), "draft")!;
  expect(after.tabs.find((tab) => tab.id === after.active)?.kind).toBe("draft");
});

test("a proposed workflow still lists with its facts and an Approve", () => {
  const row = needsYou(jobWith({ change: 7, workflow: { name: "nightly-sweep", scope: "repository" }, text: '```json\n{"steps":[]}\n```' }));
  expect(row, "the draft is listed in Needs you").toBeDefined();
  expect(row!.card?.facts.map((fact) => fact.label)).toEqual(["Name", "Scope", "Definition"]);
  expect(row!.card?.facts[0]?.value).toBe("nightly-sweep");
  expect(row!.options?.map((option) => option.label)).toEqual(["Approve"]);
  row!.onOpen();
  expect(draftTabs()).toHaveLength(0);
});
