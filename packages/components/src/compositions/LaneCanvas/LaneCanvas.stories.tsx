import { useState } from "react";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, waitFor, within } from "storybook/test";

import { LaneCanvas } from "./LaneCanvas";
import type { LanePick } from "./LaneCanvas";
import { editorLanes, jobLanes } from "./build";
import { DEFAULT } from "./model";
import type { CarriedTicket, Instance, Lifecycle, Workspace } from "./model";

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

// ---- A Job that started other Jobs --------------------------------------------------------------------------------------------

const JOB_LIFECYCLE = (columns: number): Lifecycle => ({
  id: "job",
  label: "Job",
  from: "Armada",
  kind: "job",
  states: Array.from({ length: columns }, (_, col) => ({ id: `s${col}`, label: `State ${col}`, reads: [], col, ...(col === columns - 1 ? { end: true as const } : {}) })),
  next: Array.from({ length: columns - 1 }, (_, col) => [`s${col}`, `s${col + 1}`] as const),
});
const TICKET_LIFECYCLE: Lifecycle = {
  id: "ticket",
  label: "Ticket",
  from: "Tracker",
  kind: "tracker",
  states: [{ id: "todo", label: "To do", reads: [], col: 0 }],
  next: [],
};

/** The Job's lanes for the instances given, with the workflow not started so nothing of it stands. */
function startedJobs(instances: readonly Instance[], columns = 2) {
  return jobLanes(
    {
      workspace: "my-store",
      copy: { steps: [{ id: "build", label: "Build", col: 0 }], lifecycles: [JOB_LIFECYCLE(columns), TICKET_LIFECYCLE], rules: [] },
      instances,
      fired: [],
      changes: [],
    },
    { at: -1, workflow: "feature" },
  );
}

const carrying = (title: string, ticket: CarriedTicket): Instance => ({ id: `job:${title}`, lifecycle: "job", label: title, state: "s0", visited: ["s0"], ticket });

/** The glyph an element holds after the ticket's own, by the name lucide gives its class: a glyph has no role or text to read it by. */
const glyphs = (node: HTMLElement): string[] => Array.from(node.querySelectorAll("svg")).flatMap((svg) => Array.from(svg.classList).filter((name) => name.startsWith("lucide-") && name !== "lucide"));

function Carrying({ tickets }: { tickets: Record<string, CarriedTicket> }) {
  const { lanes, rules } = startedJobs(Object.entries(tickets).map(([title, ticket]) => carrying(title, ticket)));
  return (
    <div style={{ position: "relative", height: "calc(var(--space-12) * 24)" }}>
      <LaneCanvas label="Workflow" lanes={lanes} rules={rules} />
    </div>
  );
}

const STATES: Record<string, CarriedTicket> = {
  "Job one": { label: "ARM-1", title: "Cart totals are a cent off", state: "To do", stateId: "todo" },
  "Job two": { label: "ARM-2", title: "Empty wishlist link 404s", state: "In progress", stateId: "in_progress" },
  "Job three": { label: "ARM-3", title: "Receipt prints twice", state: "In review", stateId: "in_review" },
  "Job four": { label: "ARM-4", title: "Coupon stacks with itself", state: "Closed", stateId: "closed" },
  "Job five": { label: "ARM-5", title: "Tax rounds the wrong way", state: "Done", stateId: "done" },
  "Job six": { label: "ARM-6", title: "Address field drops a digit", state: "Blocked", stateId: "blocked" },
  "Job seven": { label: "ARM-7", title: "Totals show no currency", state: "Not read", stateId: "" },
};

/** The card of a Job started for a ticket draws the ticket's key and one glyph for the state it stands in. A state no glyph names, or none, reads as unknown. */
export const TicketStateOnAJobCard: StoryObj<typeof Carrying> = {
  render: () => <Carrying tickets={STATES} />,
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const glyph: Record<string, string> = {
      "Job one": "lucide-circle-dashed",
      "Job two": "lucide-circle-dot",
      "Job three": "lucide-eye",
      "Job four": "lucide-circle-check",
      "Job five": "lucide-circle-check",
      "Job six": "lucide-circle-question-mark",
      "Job seven": "lucide-circle-question-mark",
    };
    for (const [title, ticket] of Object.entries(STATES)) {
      const card = await canvas.findByRole("button", { name: `${title}: State 0, ticket ${ticket.label}, ${ticket.state}` });
      // The ticket glyph, the key, then the state's glyph, and no pin: the card's own mark is not this one.
      const line = card.querySelector<HTMLElement>(".armada-lane-item__ticket")!;
      await expect(line.textContent).toBe(ticket.label);
      await expect(glyphs(line)).toEqual(["lucide-ticket", glyph[title]]);
    }
  },
};

/** A hover on the card names the ticket, the state it stands in, and the Job that holds it. */
export const TicketStateTooltip: StoryObj<typeof Carrying> = {
  render: () => <Carrying tickets={{ "Fix cart rounding": STATES["Job three"]! }} />,
  play: async ({ canvasElement }) => {
    const card = await within(canvasElement).findByRole("button", { name: "Fix cart rounding: State 0, ticket ARM-3, In review" });
    await userEvent.hover(card);
    // The bubble is drawn outside the canvas and hidden from the tree, so it is read by its own lines.
    await waitFor(() => expect(document.querySelectorAll(".armada-lane-tip__line").length).toBeGreaterThan(0));
    const lines = Array.from(document.querySelectorAll(".armada-lane-tip__line")).map((line) => line.textContent);
    await expect(lines.slice(-3)).toEqual(["Ticket ARM-3: Receipt prints twice", "Ticket state: In review", "Held by Fix cart rounding"]);
  },
};

/** A ticket not read yet by the Job that holds it has a key and no state: the glyph is the unknown one, the name and the tooltip say no state, and no title is made up. */
export const TicketNotReadYet: StoryObj<typeof Carrying> = {
  render: () => <Carrying tickets={{ "Fix cart rounding": { label: "ARM-8", state: "", stateId: "" } }} />,
  play: async ({ canvasElement }) => {
    const card = await within(canvasElement).findByRole("button", { name: "Fix cart rounding: State 0, ticket ARM-8" });
    await expect(glyphs(card.querySelector<HTMLElement>(".armada-lane-item__ticket")!)).toEqual(["lucide-ticket", "lucide-circle-question-mark"]);
    await userEvent.hover(card);
    await waitFor(() => expect(document.querySelectorAll(".armada-lane-tip__line").length).toBeGreaterThan(0));
    const lines = Array.from(document.querySelectorAll(".armada-lane-tip__line")).map((line) => line.textContent);
    await expect(lines.slice(-2)).toEqual(["Ticket ARM-8", "Held by Fix cart rounding"]);
  },
};

function Heads() {
  const { lanes, rules } = startedJobs([
    { id: "job:cart", lifecycle: "job", label: "Fix cart rounding", state: "s0", visited: ["s0"] },
    { id: "ticket:wishlist", lifecycle: "ticket", label: "ARM-58", state: "todo", visited: ["todo"] },
    { id: "job:old", lifecycle: "job", label: "Old attempt", state: "s1", visited: ["s0", "s1"], ended: true },
  ]);
  return (
    <div style={{ position: "relative", height: "calc(var(--space-12) * 16)" }}>
      <LaneCanvas label="Workflow" lanes={lanes} rules={rules} />
    </div>
  );
}

/** Under each lane's title is what the lane is, in its lifecycle's own name, and `Ended` once the Job ended it. */
export const HeadsSayWhatTheyAre: StoryObj<typeof Heads> = {
  render: () => <Heads />,
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const heads: [string, string][] = [["feature", "Workflow"], ["Fix cart rounding", "Job"], ["ARM-58", "Ticket"], ["Old attempt", "Ended"]];
    for (const [title, said] of heads) {
      const head = (await canvas.findByText(title)).parentElement!;
      await expect(within(head).getByText(said)).toBeVisible();
    }
    // A Job lane never reads the name of the place its states come from.
    await expect(canvas.queryByText("Armada")).toBeNull();
  },
};

function Opening({ onOpen }: { onOpen: (lane: string) => void }) {
  const { lanes, rules } = startedJobs([
    { id: "job:cart", lifecycle: "job", label: "Fix cart rounding", state: "s0", visited: ["s0"] },
    { id: "job:panel", lifecycle: "job", label: "Tidy the PR panel", state: "s0", visited: ["s0"] },
  ]);
  // Only the first has lanes of its own to open: the caller knows, and sets it.
  const drawn = lanes.map((lane) => (lane.id === "job:cart" ? { ...lane, opens: true as const } : lane));
  return (
    <div style={{ position: "relative", height: "calc(var(--space-12) * 12)" }}>
      <LaneCanvas label="Workflow" lanes={drawn} rules={rules} onOpen={(lane) => onOpen(lane.id)} />
    </div>
  );
}

/** A double-click on the head of a lane whose Job has lanes opens them, and so does Enter on it. One press with a pointer does not, and a lane with nothing to open has no button. */
export const OpenAJobsLanes: StoryObj<typeof Opening> = {
  render: (args) => <Opening {...args} />,
  args: { onOpen: fn() },
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement);
    const head = await canvas.findByRole("button", { name: "Open Workflow: Fix cart rounding" });
    await userEvent.click(head);
    await expect(args.onOpen).not.toHaveBeenCalled();
    await userEvent.dblClick(head);
    await expect(args.onOpen).toHaveBeenCalledTimes(1);
    await expect(args.onOpen).toHaveBeenLastCalledWith("job:cart");
    head.focus();
    await userEvent.keyboard("{Enter}");
    await expect(args.onOpen).toHaveBeenCalledTimes(2);
    await expect(canvas.queryByRole("button", { name: "Open Workflow: Tidy the PR panel" })).toBeNull();
    await expect(canvas.getByText("Tidy the PR panel")).toBeVisible();
  },
};

function Opened({ columns }: { columns: number }) {
  const { lanes, rules } = startedJobs(
    [
      { id: "job:early", lifecycle: "job", label: "Early", state: "s0", visited: ["s0"] },
      { id: "job:late", lifecycle: "job", label: "Late", state: `s${columns - 1}`, visited: ["s0", `s${columns - 1}`] },
    ],
    columns,
  );
  return (
    <div style={{ position: "relative", height: "calc(var(--space-12) * 12)" }}>
      <LaneCanvas label="Workflow" lanes={lanes} rules={rules} />
    </div>
  );
}

/** The canvas opens on where the Job stands. Where those states are too wide for the frame it shows the earliest, so a lane holding an early state is not left empty by one further on. */
export const OpensOnTheEarliest: StoryObj<typeof Opened> = {
  render: () => <Opened columns={14} />,
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const frame = (await canvas.findByRole("group", { name: "Workflow" })).getBoundingClientRect();
    const early = await canvas.findByRole("button", { name: "Early: State 0" });
    const late = await canvas.findByRole("button", { name: "Late: State 13" });
    await waitFor(() => {
      const at = early.getBoundingClientRect();
      expect(at.left).toBeGreaterThanOrEqual(frame.left);
      expect(at.right).toBeLessThanOrEqual(frame.right);
    });
    await expect(late.getBoundingClientRect().left).toBeGreaterThanOrEqual(frame.right);
  },
};
