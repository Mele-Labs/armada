import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";

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
