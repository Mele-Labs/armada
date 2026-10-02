// Job detail's sheets and its run sheet, through `App`: what opens one, what
// replaces one, and what a server's link sends. Moved here from the `Screens/Job
// detail` stories' sheets and run-sheet groups — #1224.

import { expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { } from "@armada/protocol";
import { escalatedGateFailure, running } from "@armada/screens/src/fixtures/build/index";
import { JOB_ID } from "@armada/screens/src/fixtures/build/base";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";

import type { BridgeApi } from "../../../shared/api";
import type { BridgeState } from "../../../shared/bridge";
import type { FleetHandle } from "./scenario";
import { onJob } from "./scenario";
import { mount, unmountAfterEach } from "./testing";

unmountAfterEach();

/** App with this Job open. `sheet` is the run sheet Fleet reads for it, and `also` more state beside. */
async function opened(
  fixture: JobFixture,
  {
    sheet,
    followed,
    also = {},
    whereOpen = false,
  }: { sheet?: BridgeState["runSheet"]; followed?: BridgeState["runFollowed"]; also?: Partial<BridgeState>; whereOpen?: boolean } = {},
): Promise<BridgeApi> {
  const scenario = onJob(fixture, { whereOpen });
  const behaves = (fleet: FleetHandle): Partial<BridgeApi> => ({
    ...(sheet === undefined
      ? {}
      : { watchRunSheet: async (jobId) => fleet.publish({ runSheet: jobId === null ? { state: "none" } : sheet }) }),
    ...(followed === undefined
      ? {}
      : { observeRun: async (_jobId, runId) => fleet.publish({ runFollowed: runId === null ? { state: "none" } : followed }) }),
  });
  const app = mount({ ...scenario, state: { ...scenario.state, ...also }, behaves });
  await expect.element(page.getByRole("button", { name: fixture.job.handle })).toBeVisible();
  return app.api;
}

const dialog = (name: string) => page.getByRole("dialog", { name });

// **Opened by its key, not by a chapter's control.** `Open the log` was the
// story chapter's own eyebrow act, and the Overview reframe of 29 Sep 2026
// took the chapters with the column they were in. `L` is `actions.toml`'s
// `open_log`, scope `detail`, and it is the route that survived — the sheet
// it opens is unchanged, which is what these claims are about.
test("L opens the activity log in a sheet", async () => {
  await opened(running());
  await userEvent.keyboard("L");
  await expect.element(dialog("Activity log")).toBeVisible();
});

// **Nothing on a Job draws a step mark any more**, so the pulse behind an
// open sheet has no subject. `WorkflowRail` drew the run tree's marks and no
// screen renders it since the Overview reframe of 29 Sep 2026 — the claim
// that the rail kept animating while a sheet was up went with it.

// `f` — `open_diff`, scope `detail`, bound on the Overview tab like `L` above.
// The control that opened the patch went with the chapters on 29 Sep 2026; the
// key and the sheet did not.
test("f opens the Job's patch in a sheet", async () => {
  await opened(running());
  await userEvent.keyboard("f");
  await expect.element(page.getByRole("dialog")).toBeVisible();
});

test("the log opens on a Job a failed Check stopped", async () => {
  await opened(escalatedGateFailure());
  await userEvent.keyboard("L");
  await expect.element(dialog("Activity log")).toBeVisible();
});

// `o` — the failed Check's output, from the open step. In the editor rather
// than a sheet, which is the whole claim; the press that also did it was the
// Checks chapter's, and the chapter is gone.
test("o opens the failed Check's output in the editor rather than a sheet", async () => {
  const api = await opened(escalatedGateFailure());
  const openArtifact = vi.spyOn(api, "openArtifact");
  await userEvent.keyboard("o");
  await expect.poll(() => openArtifact.mock.calls.length).toBe(1);
  expect(openArtifact.mock.calls[0]![0]).toBe(JOB_ID);
});

// **A Check's output is read on Record, in the row, since #1537** — and the
// Checks chapter that opened it in a sheet came off with the Overview reframe
// on 29 Sep 2026. Two claims stood here: a kept Check's row opening the sheet
// and Escape closing it, and a running one opening the sheet on the same
// press while asking main to follow the log. Record's own rows are where both
// are made now; the sheet they used has no control left that opens it.

// **Pulse is a destination, so there is no Details to press.** The Overview
// region had an eyebrow act that put the full reading in a dialog; the card
// that replaced it opens the Pulse tab, and the tab draws the reading whole.

/** This repository's own `armada.yml`, read as the Job's frozen Manifest. */
const ARMADA_RUN_SHEET_READ = {
  state: "read" as const,
  jobId: JOB_ID,
  sheet: {
    job_id: JOB_ID,
    setup: [
      { name: "bootstrap", run: "pnpm install --frozen-lockfile", narrows: false, requires: [], expect_exit_code: 0, destructive: false, frozen: true },
      { name: "browsers", run: "pnpm -C packages/components exec playwright install chromium --only-shell", narrows: false, requires: [], expect_exit_code: 0, destructive: false, frozen: true },
    ],
    checks: [
      { name: "build", run: "cargo build --workspace --locked", narrows: true, narrow_run: "cargo build --locked -p screens", requires: [], expect_exit_code: 0, destructive: false, frozen: true },
      { name: "test", run: "cargo nextest run --workspace --exclude acceptance", narrows: true, narrow_run: "cargo nextest run -p screens", requires: [], expect_exit_code: 0, destructive: false, frozen: true },
      { name: "typecheck", run: "pnpm typecheck", narrows: false, requires: [], expect_exit_code: 0, destructive: false, frozen: true },
      { name: "bridge_build", run: "pnpm -C apps/desktop build", narrows: false, requires: [], expect_exit_code: 0, destructive: false, frozen: true },
      { name: "storybook", run: "pnpm -C packages/components build-storybook", narrows: false, requires: [], expect_exit_code: 0, destructive: false, frozen: true },
      { name: "bridge_test", run: "pnpm bridge-test", narrows: false, requires: [], expect_exit_code: 0, destructive: false, frozen: true },
      { name: "format", run: "cargo fmt --all --check", narrows: true, narrow_run: "rustfmt --check --edition 2021 packages/screens/src/JobDetail.tsx", requires: ["fmt"], expect_exit_code: 0, destructive: false, frozen: true },
    ],
    commands: [
      { name: "fmt", run: "cargo fmt --all", narrows: false, requires: [], expect_exit_code: 0, destructive: false, frozen: true },
      { name: "gate", run: "cargo xtask verify-foundations", narrows: false, requires: [], expect_exit_code: 0, destructive: true, frozen: true },
    ],
    manifest_edited_at: "2026-09-08T16:40:00Z",
    worktree_on_disk: true,
    worktree_differs: false,
    drone_working: true,
  },
};

const RUN_SHEET_READ = {
  state: "read" as const,
  jobId: JOB_ID,
  sheet: {
    job_id: JOB_ID,
    setup: [],
    checks: [
      {
        name: "cargo_nextest",
        run: "cargo nextest run --workspace",
        narrows: false,
        requires: [],
        expect_exit_code: 0,
        destructive: false,
        frozen: true,
      },
    ],
    commands: [],
    worktree_on_disk: true,
    worktree_differs: false,
    drone_working: false,
  },
};


test("r opens the run sheet on this repository's own Setup, Checks and Commands", async () => {
  await opened(running(), { sheet: ARMADA_RUN_SHEET_READ });
  await userEvent.keyboard("r");
  await expect.element(dialog("Run").getByText("bridge_test")).toBeVisible();
});

test("the run sheet replaces the log: one sheet at a time", async () => {
  await opened(running());
  await userEvent.keyboard("L");
  await expect.element(dialog("Activity log")).toBeVisible();
  await userEvent.keyboard("r");
  await expect.element(dialog("Run")).toBeVisible();
  expect(dialog("Activity log").query()).toBeNull();
});

// **`Run it here` was the refused Check's row act**, and the Checks chapter
// it sat on came off with the Overview reframe. The sheet it opened is live
// and `r` opens it — what is gone is the Check arriving already selected,
// which is Journey 9's own claim and has no control left to make it.
test("r opens the run sheet, and it lists the Checks this Job can run", async () => {
  await opened(escalatedGateFailure(), { sheet: RUN_SHEET_READ });
  await userEvent.keyboard("r");
  await expect.element(dialog("Run")).toBeVisible();
  await expect.element(dialog("Run").getByText("cargo_nextest").first()).toBeVisible();
});

test("a Check running from the sheet streams its output as it prints", async () => {
  await opened(escalatedGateFailure(), {
    sheet: {
      ...RUN_SHEET_READ,
      sheet: {
        ...RUN_SHEET_READ.sheet,
        running: {
          id: "run-1",
          job_id: JOB_ID,
          name: "cargo_nextest",
          command: "cargo nextest run --workspace",
          narrowed: false,
          started_at: "2026-09-11T14:05:00Z",
        },
      },
    },
    followed: {
        state: "following",
        jobId: JOB_ID,
        runId: "run-1",
        name: "cargo_nextest",
        path: ".armada/runs/run-1/output.log",
        fromLine: 1,
        lines: ["running 2034 tests", "test settings::selectors::visible_manifests_memoises ... FAIL"],
    },
  });
  await userEvent.keyboard("r");
  await expect.element(dialog("Run").getByText(/FAIL/)).toBeVisible();
});

// **A server's row is Studio's, and `studio-server.test.tsx` makes this
// claim there.** It was also on job detail's *Where things are*, which the
// Overview reframe removed; the link handing its address to the system
// browser rather than navigating is one behaviour with one place left to
// press it.
