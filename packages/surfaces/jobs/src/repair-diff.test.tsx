// A fix's file opens its diff: the repair branch against the Job's, in the sheet the Job's own diff
// opens in. Without the surface's reader the files are names.

import { afterEach, expect, test, vi } from "vitest";
import { page } from "vitest/browser";

import type { JobTrigger, Work } from "@armada/protocol";
import { RepairFileContext, RepairNode } from "@armada/components";
import { mount, unmount } from "@armada/screens/src/mounted";

import { RepairDiffSheet } from "./repair-diff-sheet";

afterEach(() => unmount());

const held: JobTrigger = {
  name: "deploy_qa",
  when: "pr_opened",
  step: "handoff",
  level: "machine",
  state: "fix_ready",
  exit_code: 1,
  started_at: "2026-10-07T10:00:00.000Z",
  repair: { attempt: 1, branch: "armada/repair-deploy_qa-1", files: ["deploy/qa.sh", "qa.env"] },
};

const patch = (path: string, line: string) => `diff --git a/${path} b/${path}\n--- a/${path}\n+++ b/${path}\n@@ -1,1 +1,2 @@\n context\n+${line}\n`;
const work: Work = {
  files: [
    { path: "deploy/qa.sh", change: "modified" },
    { path: "qa.env", change: "modified" },
  ],
  measured_from: "armada/job-2",
  measured_whole: true,
  plan_declared: false,
  patch: patch("deploy/qa.sh", "set -e") + patch("qa.env", "QA=1"),
};

test("a file of the fix is a button where the surface can read the diff, and a name where it cannot", async () => {
  const open = vi.fn();
  mount(
    <RepairFileContext.Provider value={open}>
      <RepairNode trigger={held} />
    </RepairFileContext.Provider>,
  );
  await page.getByRole("button", { name: "qa.env" }).click();
  expect(open).toHaveBeenCalledWith(held, "qa.env");
  unmount();
  mount(<RepairNode trigger={held} />);
  expect(page.getByRole("button", { name: "qa.env" }).elements()).toHaveLength(0);
});

test("the sheet reads the fix by its Trigger and shows the file pressed, then the one picked from its rail", async () => {
  const read = vi.fn().mockResolvedValue({ ok: true, work });
  mount(<RepairDiffSheet jobId="job-2" open={{ trigger: held, path: "qa.env" }} read={read} floor={false} onClose={() => undefined} />);
  await expect.poll(() => read.mock.calls[0]).toEqual(["job-2", { trigger: "deploy_qa" }]);
  await expect.element(page.getByText("QA=1")).toBeVisible();
  expect(page.getByText("set -e").elements()).toHaveLength(0);
  await page.getByRole("button", { name: /deploy\/qa\.sh/ }).click();
  await expect.element(page.getByText("set -e")).toBeVisible();
});
