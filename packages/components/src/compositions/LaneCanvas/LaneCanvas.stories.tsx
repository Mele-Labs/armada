import { useState } from "react";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, waitFor, within } from "storybook/test";

import { LaneCanvas } from "./LaneCanvas";
import type { LanePick } from "./LaneCanvas";
import { editorLanes, jobLanes } from "./build";
import { DEFAULT } from "./model";
import type { Workspace } from "./model";

const STEPS = [
  { id: "plan", label: "Plan" },
  { id: "build", label: "Build" },
  { id: "ship", label: "Ship" },
];

const WORKSPACE: Workspace = {
  id: "my-store",
  name: "my-store",
  repositories: [{ id: "store-api", name: "store-api" }],
  lifecycles: [
    {
      id: "ticket",
      label: "Ticket",
      from: "Jira",
      states: [
        { id: "todo", label: "To do", reads: [{ source: "Jira", values: ["To Do"], scope: DEFAULT }] },
        { id: "done", label: "Done", reads: [{ source: "Jira", values: ["Done"], scope: DEFAULT }], end: true },
      ],
      next: [["todo", "done"]],
    },
  ],
  workflows: { feature: { steps: { plan: { col: 0 }, build: { col: 1 }, ship: { col: 2 } }, states: { "ticket/todo": 0, "ticket/done": 2 } } },
  rules: [],
};

/** The canvas holds the pick the way its host does, and fills the height it is given. A panel opens over its right edge once something is picked, as the side panel does. */
function Covered() {
  const [picked, setPicked] = useState<LanePick | undefined>();
  const [slot, setSlot] = useState<HTMLElement | null>(null);
  const { lanes, rules } = editorLanes({ workspace: WORKSPACE, workflow: "feature", steps: STEPS });
  return (
    <>
      <LaneCanvas
        label="feature workflow"
        lanes={lanes}
        rules={rules}
        picked={picked}
        onPick={setPicked}
        coveredBy={slot}
        editing={{ onEdit: fn(), onTell: fn(), onAddAfter: fn(), onDelete: fn(), onConnect: fn() }}
      />
      {picked === undefined ? null : (
        <aside data-side="" aria-label="Panel" style={{ position: "absolute", insetBlock: 0, insetInlineEnd: 0, width: "var(--w-dialog-wide)", background: "var(--surface-card)" }}>
          <div ref={setSlot} />
        </aside>
      )}
    </>
  );
}


function Held({ deletable, edited = true }: { deletable: boolean; edited?: boolean }) {
  const [picked, setPicked] = useState<LanePick | undefined>();
  const { lanes, rules } = editorLanes({ workspace: WORKSPACE, workflow: "feature", steps: STEPS });
  return (
    <LaneCanvas
      label="feature workflow"
      lanes={lanes}
      rules={rules}
      picked={picked}
      onPick={setPicked}
      editing={edited ? { canDelete: () => deletable, onEdit: fn(), onTell: fn(), onAddAfter: fn(), onDelete: fn(), onConnect: fn() } : undefined}
    />
  );
}

const meta: Meta<typeof Held> = {
  title: "Compositions/Lane canvas",
  component: Held,
  decorators: [
    (Story) => (
      <div style={{ position: "relative", width: "100%", height: "calc(var(--space-12) * 8)" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;
type Story = StoryObj<typeof Held>;

/** A workflow over the lifecycle it moves: its steps on top, and a lane of states under them. Read, never edited, a node is a button that picks it and nothing stands over it. */
export const Read: Story = {
  args: { deletable: true, edited: false },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.click(await canvas.findByRole("button", { name: "Step: Build" }));
    await expect(canvas.queryByRole("toolbar")).toBeNull();
  },
};

/** What a tab added can be deleted, and the toolbar over it says so beside Edit, Tell an agent and Add after. */
export const OwnStep: Story = {
  args: { deletable: true },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.click(await canvas.findByRole("button", { name: "Step: Build" }));
    const toolbar = await canvas.findByRole("toolbar", { name: "Edit step Build" });
    await expect(within(toolbar).getByRole("button", { name: "Add after step Build" })).toBeVisible();
    await expect(within(toolbar).getByRole("button", { name: "Delete step Build" })).toBeVisible();
  },
};

/** What a level above holds is not deleted below it: no Delete, while Add after and a connector from the node are still there. */
export const InheritedStep: Story = {
  args: { deletable: false },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.click(await canvas.findByRole("button", { name: "Step: Build" }));
    const toolbar = await canvas.findByRole("toolbar", { name: "Edit step Build" });
    await expect(within(toolbar).getByRole("button", { name: "Add after step Build" })).toBeVisible();
    await expect(within(toolbar).queryByRole("button", { name: "Delete step Build" })).toBeNull();
    await expect(canvas.getByRole("button", { name: "Connect from step Build" })).toBeInTheDocument();
  },
};

/** A panel opens over the right edge on a pick. The canvas pans, and never reflows, so the node and the toolbar over it stand in what the panel leaves. */
export const BesideAPanel: Story = {
  render: () => <Covered />,
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.click(await canvas.findByRole("button", { name: "Step: Ship" }));
    const panel = await canvas.findByRole("complementary", { name: "Panel" });
    const toolbar = await canvas.findByRole("toolbar", { name: "Edit step Ship" });
    const node = canvas.getByRole("button", { name: "Step: Ship" });
    const edge = panel.getBoundingClientRect().left;
    await waitFor(() => expect(node.getBoundingClientRect().right).toBeLessThanOrEqual(edge));
    await expect(toolbar.getBoundingClientRect().right).toBeLessThanOrEqual(edge);
  },
};

/** A Job's ticket: its lane head is a button, and its menu offers Teach (the ContextMenu key here, as the right press does with a pointer). */
function Taught({ onTeach }: { onTeach: (title: string) => void }) {
  const { lanes, rules } = jobLanes(
    {
      workspace: "my-store",
      copy: {
        steps: [{ id: "build", label: "Build", col: 0 }, { id: "ship", label: "Ship", col: 1 }],
        lifecycles: [{ id: "ticket", label: "Ticket", from: "Tracker", kind: "tracker", states: [{ id: "todo", label: "To do", reads: [], col: 0 }, { id: "done", label: "Done", reads: [], col: 1, end: true }], next: [["todo", "done"]] }],
        rules: [],
      },
      instances: [{ id: "ticket:ARM-41", lifecycle: "ticket", label: "ARM-41", state: "todo", visited: ["todo"] }],
      fired: [],
      changes: [],
    },
    { at: 0, workflow: "feature", working: true },
  );
  return <LaneCanvas label="Workflow" lanes={lanes} rules={rules} onTeach={(lane) => onTeach(lane.title)} />;
}

export const TeachOnATicket: StoryObj<typeof Taught> = {
  render: (args) => <Taught {...args} />,
  args: { onTeach: fn() },
  play: async ({ canvasElement, args }) => {
    const head = await within(canvasElement).findByRole("button", { name: "Ticket ARM-41" });
    head.focus();
    await userEvent.keyboard("{ContextMenu}");
    await userEvent.click(await within(document.body).findByRole("menuitem", { name: "Teach" }));
    await expect(args.onTeach).toHaveBeenCalledWith("ARM-41");
  },
};
