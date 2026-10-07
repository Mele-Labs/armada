import type { Meta, StoryObj } from "@storybook/react-vite";
import type { JobTrigger, TriggerSummary } from "@armada/protocol";
import { expect, fn, userEvent, within } from "storybook/test";

import { FiredTriggers, TriggerRows } from "./WorkflowTriggers";

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

/** A Job's Triggers: the state is a mark with a tooltip, never a phrase, and a firing with a log line goes to it. */
export const Fired: Story = {
  render: () => <FiredTriggers triggers={FIRED} onOpenLog={fn()} />,
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
