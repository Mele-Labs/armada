import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";

import type { TriggerSummary } from "@armada/protocol";

import type { Entry } from "./def";
import { SOURCE_OF_WIRE, entriesOf, readDefinition } from "./json";
import { MOCK_FILES, leftOutRowsOf, workflowRowsOf } from "./mock";
import { WorkflowCreator } from "./WorkflowCreator";

const CURRENT = "01M1CNPKTV0018H2M1CXDNBK06";

const entries = entriesOf(workflowRowsOf(MOCK_FILES), leftOutRowsOf(MOCK_FILES));

/** What Fleet answers for one row: the file's text, read the way the app reads it. */
async function onRead(entry: Entry) {
  const held = MOCK_FILES.find((one) => one.id === entry.id && SOURCE_OF_WIRE[one.source] === entry.source);
  return held === undefined ? { ok: false as const, said: "Not held" } : readDefinition(held.text, entry.source, CURRENT);
}

const meta: Meta<typeof WorkflowCreator> = {
  title: "Compositions/Workflow creator",
  component: WorkflowCreator,
  args: {
    repository: CURRENT,
    manifests: [
      { id: CURRENT, name: "armada" },
      { id: "01M1CNPKTV0018H2M1CXDNBK07", name: "ledger" },
    ],
    entries,
    onRead,
    onSave: async () => ({ ok: true }),
    onDiscuss: fn(),
  },
  decorators: [
    (Story) => (
      <div style={{ background: "var(--surface-canvas)", minHeight: "var(--h-workflow-canvas)" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof WorkflowCreator>;

/** Every file Fleet holds, a row each: carried, Kit, repository, the ones a more specific place replaced, and one left out. */
export const List: Story = {};

/** A workflow opened as its graph, a step sending work back drawn as a back edge. */
export const Open: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.click(await canvas.findByRole("button", { name: "bug, repository" }));
    await expect(await canvas.findByText("up to 4 passes")).toBeVisible();
  },
};

/** A carried workflow saved to the repository, where one of that id is already there: Fleet's refusal in the words it gave, replaced only by a second Save. */
export const Refused: Story = {
  args: {
    onSave: fn(async (_def, overwrite: boolean) =>
      overwrite ? { ok: true as const } : { ok: false as const, said: "bug.json already defines `feature` and nothing was written", exists: true as const },
    ),
  },
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement);
    await userEvent.click(await canvas.findByRole("button", { name: "feature, carried" }));
    await userEvent.click(await canvas.findByRole("button", { name: /^Save$/ }));
    await expect(await canvas.findByText(/already defines `feature`/)).toBeVisible();
    await expect(args.onSave).toHaveBeenLastCalledWith(expect.anything(), false);
  },
};

/** What `list_triggers` answers: one for every workflow at every step passing, and one for `feature` alone. */
const TRIGGERS: TriggerSummary[] = [
  { name: "fmt", when: "step_passes", runs: { kind: "command", name: "fmt" }, block: false, repair: false, level: "repository", file: ".armada/triggers/fmt.yml" },
  { name: "docs-check", when: "step_passes", step: "implement", workflow: "feature", runs: { kind: "skill", name: "docs-check" }, block: false, repair: false, level: "repository", file: ".armada/triggers/docs-check.yml" },
];

/** A Trigger set for every workflow is a leaf off each step it fires at, on every workflow's canvas, and there is no panel for them apart from the workflows. */
export const EveryWorkflow: Story = {
  args: {
    triggers: {
      triggers: TRIGGERS,
      commands: ["fmt"],
      onOpen: fn(async () => ({ ok: false as const, said: "Not read" })),
      onSave: fn(async () => ({ ok: false as const, said: "Not saved" })),
      onRemove: fn(async () => ({ ok: false as const, said: "Not removed" })),
    },
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.click(await canvas.findByRole("button", { name: "feature, carried" }));
    // Four steps, a leaf off each; the Trigger set for this workflow alone is a line on its step.
    await expect(await canvas.findAllByRole("img", { name: "Every workflow" })).toHaveLength(4);
    await expect(await canvas.findByText("On pass")).toBeVisible();
    await expect(canvas.queryByRole("region", { name: "Triggers on every workflow" })).toBeNull();
    await userEvent.click(await canvas.findByRole("button", { name: "bug, repository" }));
    await expect(await canvas.findAllByRole("img", { name: "Every workflow" })).toHaveLength(3);
    await expect(canvas.queryByText("On pass")).toBeNull();
  },
};
