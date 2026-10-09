import type { Meta, StoryObj } from "@storybook/react-vite";
import type { JobTrigger, TriggerSummary } from "@armada/protocol";
import { expect, fn, userEvent, within } from "storybook/test";

import { HoldNode, holdsOf, JobAlertMark, RepairNode, RepairPrMark, TriggerAlertMark, TriggerLeaf, TriggerRows } from "./WorkflowTriggers";

/** What `list_triggers` answers: the machine's copy of `gate` over the repository's, and one the repository cannot run. */
const LISTED: TriggerSummary[] = [
  { name: "fmt", when: "step_passes", step: "implement", runs: { kind: "command", name: "fmt" }, block: false, repair: false, level: "repository", file: ".armada/triggers/fmt.yml" },
  {
    name: "gate",
    when: "pr_opened",
    runs: { kind: "command", name: "gate" },
    block: false,
    repair: true,
    level: "machine",
    file: "~/.armada/machine/triggers/gate.yml",
    overrides: [{ level: "repository", file: ".armada/triggers/gate.yml" }],
  },
  {
    name: "deploy_qa",
    when: "pr_opened",
    runs: { kind: "command", name: "deploy_qa" },
    block: false,
    repair: false,
    level: "machine",
    file: "~/.armada/machine/triggers/deploy_qa.yml",
    skipped: { reason: "not_in_this_repo", name: "deploy_qa", said: "`deploy_qa` is not a Command this repository declares" },
  },
];

const meta: Meta<typeof TriggerRows> = {
  title: "Compositions/Workflow triggers",
  component: TriggerRows,
  args: { triggers: LISTED, label: "Triggers", onOpen: fn() },
};
export default meta;

type Story = StoryObj<typeof TriggerRows>;

/** Each marked by the level that sets it, the copy it replaced beneath it struck through, and a skipped one marked. */
export const Rows: Story = {
  play: async ({ canvasElement, args }) => {
    const rows = within(canvasElement);
    await expect(rows.getByRole("button", { name: /gate, PR opened, this machine$/ })).toBeVisible();
    await expect(rows.getByRole("img", { name: "`deploy_qa` is not a Command this repository declares" })).toBeVisible();
    await userEvent.click(rows.getByRole("button", { name: /gate, PR opened, this repository, replaced/ }));
    await expect(args.onOpen).toHaveBeenCalledWith(LISTED[1], "repository");
  },
};

const AT = "2026-10-07T10:00:00.000Z";

/** What `JobDetail.triggers` carries: one in each state a firing can be in. */
const FIRED: JobTrigger[] = [
  { name: "fmt", when: "step_passes", step: "implement", level: "repository", state: "passed", exit_code: 0, started_at: AT, ended_at: AT, log_at: AT },
  { name: "gate", when: "pr_opened", step: "handoff", level: "machine", state: "failed", exit_code: 1, started_at: AT, ended_at: AT, log_at: AT },
  {
    name: "deploy_qa",
    when: "pr_opened",
    step: "handoff",
    level: "machine",
    state: "skipped",
    skipped: { reason: "not_in_this_repo", name: "deploy_qa", said: "`deploy_qa` is not a Command this repository declares" },
    started_at: AT,
    ended_at: AT,
  },
  { name: "wipe_qa", when: "pr_opened", step: "handoff", level: "repository", state: "awaiting_owner", started_at: AT },
  { name: "smoke", when: "step_starts", step: "handoff", level: "repository", state: "running", started_at: AT },
  { name: "lint_docs", when: "step_passes", step: "handoff", level: "repository", state: "pending" },
];

/** Each Trigger set for every workflow wears the same mark, which says so in its tooltip. */
export const EveryWorkflow: Story = {
  play: async ({ canvasElement }) => {
    const rows = within(canvasElement);
    await expect(rows.getAllByRole("img", { name: "Every workflow" })).toHaveLength(LISTED.length);
  },
};

/** A Job's Triggers, a leaf each off its step: the state is a mark with a tooltip, never a phrase, and a firing with a log line goes to it. */
export const Fired: Story = {
  render: () => (
    <>
      {FIRED.map((one) => (
        <TriggerLeaf key={one.name} trigger={one} onOpenLog={fn()} />
      ))}
    </>
  ),
  play: async ({ canvasElement }) => {
    const rows = within(canvasElement);
    for (const name of ["Passed", "Failed, exit 1", "Waiting on you", "Running", "Pending"]) {
      await expect(rows.getByRole("img", { name })).toBeVisible();
    }
    await expect(rows.getByRole("img", { name: "`deploy_qa` is not a Command this repository declares" })).toBeVisible();
    await expect(rows.getByRole("button", { name: "fmt" })).toBeVisible();
    await expect(rows.queryByRole("button", { name: "smoke" })).toBeNull();
  },
};

/** `deploy_qa` failed after the pull request opened, and a repair is on it. */
const REPAIRED: JobTrigger = {
  name: "deploy_qa",
  when: "pr_opened",
  step: "handoff",
  level: "machine",
  state: "fix_ready",
  exit_code: 1,
  started_at: AT,
  log_at: AT,
  repair: { attempt: 1, branch: "armada/repair-deploy_qa-1", files: ["deploy/qa.sh", ".armada/qa.env"] },
};

/** A fix held for the owner: the files it changes beside the choice, which Fleet never makes. */
export const RepairAsks: Story = {
  render: () => <RepairNode trigger={REPAIRED} onChoose={fn().mockResolvedValue({ ok: true })} />,
  play: async ({ canvasElement }) => {
    const branch = within(canvasElement);
    await expect(branch.getByRole("img", { name: "Repair branch" })).toBeVisible();
    await expect(branch.getByRole("list", { name: "The fix" })).toBeVisible();
    await expect(branch.getByRole("group", { name: "Where the fix goes" })).toBeVisible();
  },
};

/** Each state of a repair is a mark with a tooltip, and the two ends are marks of their own. */
export const RepairEnds: Story = {
  render: () => (
    <>
      <RepairNode trigger={{ ...REPAIRED, state: "repairing", repair: { attempt: 1 } }} />
      <RepairNode trigger={{ ...REPAIRED, state: "failed", repair: { attempt: 2 } }} />
      <RepairNode trigger={{ ...REPAIRED, state: "passed", repair: { ...REPAIRED.repair!, choice: "new_pr", pull_request: { url: "https://forge.test/pull/1751", number: 1751 } } }} />
      <RepairPrMark trigger={{ ...REPAIRED, state: "passed", repair: { ...REPAIRED.repair!, choice: "new_pr", pull_request: { url: "https://forge.test/pull/1751", number: 1751 } } }} />
      <TriggerAlertMark />
    </>
  ),
  play: async ({ canvasElement }) => {
    const ends = within(canvasElement);
    for (const name of ["Repair Drone working", "Failed", "Passed", "New PR", "Alert"]) {
      await expect(ends.getAllByRole("img", { name })[0]).toBeVisible();
    }
    await expect(ends.getByText("#1751")).toBeVisible();
  },
};

const HELD: JobTrigger = { name: "deploy_qa", when: "pr_opened", step: "handoff", level: "machine", state: "held", exit_code: 1, blocks: true };

/** A blocking Trigger that failed holds the Job: the barrier, the name, Rerun and Skip, and the bell the Board row carries. */
export const TriggerHolds: Story = {
  render: () => (
    <>
      <HoldNode held={holdsOf([HELD])[0]!} onAct={fn().mockResolvedValue({ ok: true })} />
      <JobAlertMark alert={{ kind: "held", trigger: "deploy_qa", when: "pr_opened", step: "handoff" }} />
    </>
  ),
  play: async ({ canvasElement }) => {
    const hold = within(canvasElement);
    await expect(hold.getByRole("group", { name: "deploy_qa, PR opened, handoff" })).toBeVisible();
    await expect(hold.getByRole("button", { name: "Rerun" })).toBeEnabled();
    await expect(hold.getByRole("button", { name: "Skip" })).toBeEnabled();
    await expect(hold.getAllByRole("img", { name: "Held, deploy_qa, PR opened, handoff" })).toHaveLength(2);
  },
};

const ASKING: JobTrigger = { name: "wipe_qa", when: "pr_opened", step: "handoff", level: "repository", state: "awaiting_owner", blocks: true };

/** A Trigger on a destructive Command waits on him: its leaf has Run and Skip, and the Board row's bell names it. A press is sent once as the verb Fleet answers to. */
export const TriggerAsks: Story = {
  render: () => (
    <>
      <TriggerLeaf trigger={ASKING} onAct={fn().mockResolvedValue({ ok: true })} />
      <JobAlertMark alert={{ kind: "asks", trigger: "wipe_qa", when: "pr_opened", step: "handoff" }} />
    </>
  ),
  play: async ({ canvasElement }) => {
    const leaf = within(canvasElement);
    await expect(leaf.getByRole("group", { name: "wipe_qa, PR opened" })).toBeVisible();
    await expect(leaf.getByRole("button", { name: "Run" })).toBeEnabled();
    await expect(leaf.getByRole("button", { name: "Skip" })).toBeEnabled();
    await expect(leaf.queryByRole("button", { name: "Rerun" })).toBeNull();
    await expect(leaf.getByRole("img", { name: "Waiting on you, wipe_qa, PR opened, handoff" })).toBeVisible();
    await userEvent.click(leaf.getByRole("button", { name: "Run" }));
    await expect(leaf.getByRole("button", { name: "Run" })).toBeDisabled();
  },
};
