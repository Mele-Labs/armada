// Pause and Resume on Cleanup's tiles: when each is offered, that Pause asks
// first and Resume does not, and that each refusal is said in the panel of the
// tile it was about, in git's words.

import { afterEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { JobSummary, Outcome, WorktreeHeld, WorktreeSlot } from "@armada/protocol";

import { mount, unmount } from "./mounted";
import { Worktrees } from "./Worktrees";

afterEach(unmount);

const WANT = (): void => {};
const NOW = Date.parse("2026-10-06T10:00:00Z");

const job = (over: Partial<JobSummary> = {}): JobSummary => ({
  id: "01JOB",
  handle: "2-debounce",
  title: "Debounce the Job Board",
  status: "running",
  workflow_id: "bug",
  owner_manifest_id: "armada",
  origin: "dispatched",
  urgency: "normal",
  atomic: false,
  model: "sonnet",
  created_at: "2026-10-06T09:00:00Z",
  branch: "armada/2-debounce",
  ...over,
});

const bay: WorktreeSlot = {
  manifest_id: "armada",
  slot: 3,
  path: "/Users/user/armada/.armada/slots/slot-3",
  base: "main",
  warm: true,
  branch: "armada/2-debounce",
  held: { state: "job", job_id: "01JOB", job_title: "Debounce the Job Board" },
};

const tree = (over: Partial<WorktreeHeld> = {}): WorktreeHeld => ({
  job_id: "01JOB",
  job_title: "Debounce the Job Board",
  status: "running",
  last_moved_at: "2026-10-06T09:50:00Z",
  path: bay.path,
  branch: "armada/2-debounce",
  held: [{ why: "uncommitted", files: ["src/a.rs"] }, { why: "not_terminal", status: "running" }],
  on_disk: true,
  ...over,
});

const refused = (code: string, message = "git said no"): Outcome => ({
  ok: false,
  why: "refused",
  error: { code, message, run_id: "r", fields: {}, chain: [] },
});

type Sent = { paused: string[]; resumed: string[] };

function opened(summary: JobSummary, worktree: WorktreeHeld, answer: { pause?: Outcome; resume?: Outcome } = {}): Sent {
  const sent: Sent = { paused: [], resumed: [] };
  mount(
    <Worktrees
      onWant={WANT}
      held={{ state: "read", held: { worktrees: [worktree], slots: [bay] } }}
      jobs={[summary]}
      onReclaim={async () => ({ ok: true })}
      onDeleteBranch={async () => ({ ok: true })}
      onForget={async () => ({ ok: true })}
      onPause={async (id) => (sent.paused.push(id), answer.pause ?? { ok: true })}
      onResume={async (id) => (sent.resumed.push(id), answer.resume ?? { ok: true })}
      now={NOW}
      onClose={() => {}}
      onCopied={() => {}}
      onOpenJob={() => {}}
    />,
  );
  return sent;
}

const openPanel = async () => {
  await userEvent.click(page.getByRole("button", { name: "slot-3", exact: true }));
  return page.getByRole("dialog", { name: "slot-3" });
};

test("Pause on a running Job's bay confirms first, listing what it does, and sends nothing until pressed", async () => {
  const sent = opened(job(), tree());
  const panel = await openPanel();
  await userEvent.click(panel.getByRole("button", { name: "Pause", exact: true }));
  expect(sent.paused).toEqual([]);
  const confirm = page.getByRole("group", { name: "Pause slot-3" });
  await expect.element(confirm.getByText("Stops the Drone and its processes")).toBeInTheDocument();
  await expect.element(confirm.getByText("Commits uncommitted files to branch armada/2-debounce as a WIP commit")).toBeInTheDocument();
  await expect.element(confirm.getByText("Releases slot-3")).toBeInTheDocument();
  await expect.element(confirm.getByText("The step restarts on Resume")).toBeInTheDocument();
  await userEvent.click(confirm.getByRole("button", { name: "Cancel" }));
  expect(sent.paused).toEqual([]);
  await userEvent.click(panel.getByRole("button", { name: "Pause", exact: true }));
  await userEvent.click(page.getByRole("group", { name: "Pause slot-3" }).getByRole("button", { name: "Pause" }));
  expect(sent.paused).toEqual(["01JOB"]);
});

test("a Job parked at a gate lists the release and no process", async () => {
  opened(job({ status: "awaiting_review" }), tree({ status: "awaiting_review", held: [] }));
  const panel = await openPanel();
  await userEvent.click(panel.getByRole("button", { name: "Pause", exact: true }));
  const confirm = page.getByRole("group", { name: "Pause slot-3" });
  await expect.element(confirm.getByText("Releases slot-3")).toBeInTheDocument();
  expect(confirm.getByText("Stops the Drone and its processes").elements()).toHaveLength(0);
  expect(confirm.getByText(/WIP commit/).elements()).toHaveLength(0);
});

test("a paused Job is offered Resume and not Pause, and Resume is sent at once", async () => {
  const paused = job({ status: "awaiting_review", paused: { by: "person", at: "2026-10-06T09:56:00Z", resuming: false } });
  const sent = opened(paused, tree({ status: "awaiting_review", held: [] }));
  const panel = await openPanel();
  expect(panel.getByRole("button", { name: "Pause", exact: true }).elements()).toHaveLength(0);
  await expect.element(panel.getByRole("img", { name: "Paused 4 minutes ago: work saved on branch armada/2-debounce, slot released" })).toBeInTheDocument();
  await userEvent.click(panel.getByRole("button", { name: "Resume", exact: true }));
  expect(sent.resumed).toEqual(["01JOB"]);
});

test("a Job that has ended is offered neither", async () => {
  opened(job({ status: "completed_success" }), tree({ status: "completed_success", held: [] }));
  const panel = await openPanel();
  expect(panel.getByRole("button", { name: /^(Pause|Resume)$/ }).elements()).toHaveLength(0);
});

test.each([
  ["fleet.not_pausable", "Not paused: no worktree to give back"],
  ["fleet.already_paused", "Already paused"],
  ["fleet.checks_running", "Not paused: its Checks are reading the worktree"],
  ["fleet.pause_refused", "Not paused: git said no"],
])("a Pause refused as %s says %s in the panel", async (code, said) => {
  opened(job(), tree(), { pause: refused(code) });
  const panel = await openPanel();
  await userEvent.click(panel.getByRole("button", { name: "Pause", exact: true }));
  await userEvent.click(page.getByRole("group", { name: "Pause slot-3" }).getByRole("button", { name: "Pause" }));
  await expect.element(panel.getByRole("alert")).toHaveTextContent(said);
});

test.each([
  ["fleet.not_paused", "Not paused"],
  ["fleet.pause_refused", "Not resumed: git said no"],
])("a Resume refused as %s says %s in the panel", async (code, said) => {
  const paused = job({ status: "awaiting_review", paused: { by: "person", at: "2026-10-06T09:56:00Z", resuming: false } });
  opened(paused, tree({ status: "awaiting_review", held: [] }), { resume: refused(code) });
  const panel = await openPanel();
  await userEvent.click(panel.getByRole("button", { name: "Resume", exact: true }));
  await expect.element(panel.getByRole("alert")).toHaveTextContent(said);
});
