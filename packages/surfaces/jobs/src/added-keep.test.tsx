// Keeping an added step for every Job: a Drone step takes the next free name against the Triggers
// already saved, as the Trigger editor does, and Replace is offered only where the save said the
// same Trigger is there.

import { afterEach, expect, test, vi } from "vitest";
import { page } from "vitest/browser";

import type { AddedStep, TriggerSummary } from "@armada/protocol";
import { mount, unmount } from "@armada/screens/src/mounted";

import { AddedSheets } from "./added-steps";
import type { AddedBinding, AddedSteps } from "./added-steps";

afterEach(() => unmount());

const SAVED: TriggerSummary = {
  name: "format-the-changelog",
  when: "step_passes",
  step: "verify",
  runs: { kind: "drone", brief: "Format the changelog" },
  block: false,
  repair: false,
  level: "machine",
  file: "format-the-changelog.yml",
};

const added = (runs: AddedStep["runs"]): AddedStep => ({
  id: "a1",
  runs,
  when: "step_passes",
  step: "verify",
  block: false,
  repair: false,
  placed: "running",
  added_at: "2026-10-07T10:00:00.000Z",
  state: "passed",
});

function sheet(one: AddedStep, binding: AddedBinding) {
  const stub = {
    jobId: "job-1",
    workflow: "feature",
    steps: ["plan", "verify"],
    rows: [one],
    open: null,
    onOpen: vi.fn(),
    composing: null,
    onComposing: vi.fn(),
    keeping: one,
    onKeeping: vi.fn(),
    binding,
  } as unknown as AddedSteps;
  mount(<AddedSheets added={stub} />);
}

const binding = (triggers: TriggerSummary[], onKeep: AddedBinding["onKeep"]): AddedBinding => ({
  commands: ["fmt"],
  triggers,
  onAdd: vi.fn(),
  onRemove: vi.fn(),
  onEdit: vi.fn(),
  onKeep,
});

test("a Drone step whose slug is taken is kept under the next free name, and no Replace is offered", async () => {
  const onKeep = vi.fn().mockResolvedValue({ ok: true, saved: { name: "format-the-changelog-2", level: "machine", file: "x.yml", runs_from: "machine" } });
  sheet(added({ kind: "drone", brief: "Format the changelog" }), binding([SAVED], onKeep));
  await page.getByRole("button", { name: "Keep" }).click();
  expect(onKeep).toHaveBeenCalledTimes(1);
  const [scope, definition, overwrite, from] = onKeep.mock.calls[0]!;
  expect(scope).toBe("machine");
  expect(JSON.parse(definition).name).toBe("format-the-changelog-2");
  expect(overwrite).toBe(false);
  expect(from).toEqual({ job_id: "job-1", addition_id: "a1" });
  expect(page.getByRole("button", { name: "Replace" }).elements()).toHaveLength(0);
});

test("a Drone step whose slug is free is kept under it", async () => {
  const onKeep = vi.fn().mockResolvedValue({ ok: true, saved: { name: "format-the-changelog", level: "machine", file: "x.yml", runs_from: "machine" } });
  sheet(added({ kind: "drone", brief: "Format the changelog" }), binding([], onKeep));
  await page.getByRole("button", { name: "Keep" }).click();
  expect(JSON.parse(onKeep.mock.calls[0]![1]).name).toBe("format-the-changelog");
});

test("keeping the same Trigger again is offered as Replace where the save said it is there", async () => {
  const onKeep = vi.fn().mockResolvedValue({ ok: false, said: "fmt is already saved", exists: true });
  sheet(added({ kind: "script", command: "fmt" }), binding([{ ...SAVED, name: "fmt", runs: { kind: "command", name: "fmt" } }], onKeep));
  await page.getByRole("button", { name: "Keep" }).click();
  await expect.element(page.getByRole("button", { name: "Replace" })).toBeVisible();
  await page.getByRole("button", { name: "Replace" }).click();
  expect(onKeep.mock.calls[1]![2]).toBe(true);
});
