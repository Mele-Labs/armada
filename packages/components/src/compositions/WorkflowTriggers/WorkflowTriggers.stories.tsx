import type { Meta, StoryObj } from "@storybook/react-vite";
import type { TriggerSummary } from "@armada/protocol";
import { expect, fn, userEvent, within } from "storybook/test";

import { TriggerRows } from "./WorkflowTriggers";

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

/** Each Trigger set for every workflow wears the same mark, which says so in its tooltip. */
export const EveryWorkflow: Story = {
  play: async ({ canvasElement }) => {
    const rows = within(canvasElement);
    await expect(rows.getAllByRole("img", { name: "Every workflow" })).toHaveLength(LISTED.length);
  },
};
