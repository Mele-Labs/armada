import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, waitFor } from "storybook/test";

import { WorkflowCanvas, type WorkflowCanvasEdge, type WorkflowCanvasNode } from "./WorkflowCanvas";

const meta: Meta<typeof WorkflowCanvas> = {
  title: "Compositions/Workflow canvas",
  component: WorkflowCanvas,
  parameters: { layout: "fullscreen" },
  decorators: [
    (Story) => (
      <div style={{ height: "100vh", background: "var(--surface-canvas)" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof WorkflowCanvas>;

// The four steps `feature.json` declares, one under the last, as the Workflow
// tab draws them. The placement is the caller's, and the caller here writes
// the numbers `workflow-canvas.ts` computes: a card with one row under its
// name, then a card with none, which sits a row closer to the next.
const STEP_APART = 112;
const ROW = 28;

const steps: WorkflowCanvasNode[] = [
  {
    id: "step:plan",
    position: { x: 0, y: 0 },
    card: { kind: "step", name: "Plan the change", activity: "advanced", said: "advanced", ordinal: 1, line: "6m 00s · advanced", onOpen: fn() },
  },
  {
    id: "step:implement",
    position: { x: 0, y: STEP_APART },
    card: { kind: "step", name: "Implement", activity: "running", said: "running", ordinal: 2, current: true, line: "55m · 2 Drones", bar: { groups: ["done", "done", "working", "open"], label: "2 of 4 groups done" }, onOpen: fn() },
  },
  {
    id: "step:tests",
    position: { x: 0, y: STEP_APART * 2 },
    card: { kind: "step", name: "Write tests", activity: "not_started", said: "not started", ordinal: 3, onOpen: fn() },
  },
  {
    id: "step:handoff",
    position: { x: 0, y: STEP_APART * 3 - ROW },
    card: { kind: "step", name: "Review the change", activity: "not_started", said: "not started", ordinal: 4, gate: "will ask you", onOpen: fn() },
  },
];

const spine: WorkflowCanvasEdge[] = [
  { id: "plan>implement", source: "step:plan", target: "step:implement", kind: "leads" },
  { id: "implement>tests", source: "step:implement", target: "step:tests", kind: "leads" },
  { id: "tests>handoff", source: "step:tests", target: "step:handoff", kind: "leads" },
];

/** The run as the tab lays it out: a spine running down, hung from the top. */
const RUNS_DOWN = { runsDown: true, hangsFromTop: true } as const;

/**
 * Where one edge runs, in the window's own coordinates — sampled along the
 * path, because an edge is one `<path>` and its shape is the whole question.
 */
function pointsAlong(path: SVGPathElement, samples = 60): DOMPoint[] {
  const ctm = path.getScreenCTM()!;
  const length = path.getTotalLength();
  return Array.from({ length: samples + 1 }, (_, at) =>
    path.getPointAtLength((length * at) / samples).matrixTransform(ctm),
  );
}

/**
 * A feature Job mid-implement: the four steps its workflow file declares, one
 * under the last.
 *
 * **A `play`, because a still cannot say which line is which.** An edge's
 * accessible name is where the graph says what it joins, and it is the only
 * place a reader who cannot see it is told at all.
 */
export const AFeatureRun: Story = {
  args: { nodes: steps, edges: spine, label: "The run", running: "step:implement", onFollowing: fn(), ...RUNS_DOWN },
  play: async ({ canvas }) => {
    await waitFor(() => expect(canvas.getByLabelText("Plan the change leads to Implement")).toBeInTheDocument());
    expect(canvas.getByLabelText("Write tests leads to Review the change")).toBeInTheDocument();
  },
};

/** One group of a plan, with its two tasks beside it. */
const GROUPS: { id: string; ordinal: number; said: string; activity: "advanced" | "running" | "not_started"; done: boolean; tasks: string[] }[] = [
  { id: "g1", ordinal: 1, said: "passed", activity: "advanced", done: true, tasks: ["Serve one read of everything running", "Send an event when what is running changes"] },
  { id: "g2", ordinal: 2, said: "passed", activity: "advanced", done: true, tasks: ["Reword the stat to one running beside the most", "Say the same words on the Board and in Settings"] },
  { id: "g3", ordinal: 3, said: "checking", activity: "running", done: true, tasks: ["Draw what is running, in four lists", "Open a Drone's Job from its row"] },
  { id: "g4", ordinal: 4, said: "pending", activity: "not_started", done: false, tasks: ["Say what holds the next Drone back", "Four stories: nothing out, one Drone, three at once, and the most"] },
];

const GROUP_APART = 232;
const TASK_X = 256;
const TASK_APART = 104;

const planNodes: WorkflowCanvasNode[] = GROUPS.flatMap((group, at) => [
  {
    id: `group:${group.id}`,
    position: { x: 0, y: at * GROUP_APART },
    card: {
      kind: "group" as const,
      name: `Group ${group.ordinal}`,
      activity: group.activity,
      said: group.said,
      facts: [{ value: "2 tasks" }, { value: "7 checks" }],
    },
  },
  ...group.tasks.map((title, k) => ({
    id: `task:${group.id}-${k}`,
    position: { x: TASK_X, y: at * GROUP_APART + k * TASK_APART },
    card: {
      kind: "task" as const,
      name: title,
      activity: group.done ? ("advanced" as const) : ("not_started" as const),
      said: group.done ? "done" : "open",
      facts: [{ value: `T${at * 2 + k + 1}` }, { value: "1 file" }],
      onOpen: fn(),
    },
  })),
]);

const planEdges: WorkflowCanvasEdge[] = GROUPS.flatMap((group) =>
  group.tasks.map((_, k) => ({
    id: `${group.id}>t${k}`,
    source: `group:${group.id}`,
    target: `task:${group.id}-${k}`,
    kind: "holds" as const,
  })),
);

/**
 * The same canvas drawing a plan rather than a run — the Plan tab's Graph view
 * (owner, 25 Sep 2026). **A group is a root here**: there are no step nodes on
 * that tab, so the plan is as many small trees as it has groups.
 */
export const APlan: Story = {
  args: { nodes: planNodes, edges: planEdges, label: "The plan" },
  play: async ({ canvas }) => {
    await waitFor(() =>
      expect(canvas.getByLabelText("Group 1 holds Serve one read of everything running")).toBeInTheDocument(),
    );
    // A task is what a press opens; a group is a card and not a control.
    expect(canvas.getByRole("button", { name: "Draw what is running, in four lists, done" })).toBeInTheDocument();
    expect(canvas.queryByRole("button", { name: "Group 4, pending" })).toBeNull();
  },
};

/**
 * A step that sends the run back to an earlier one — dashed, and returning
 * along the spine's right side.
 *
 * **No shipped workflow declares one today.** `verdict_routing_target` is on
 * the wire and none of the eight files under `.armada/workflows/` sets it, so
 * this is the state drawn ahead of the data rather than a reading of it.
 *
 * **A `play`, because a still cannot say which line runs *under* a card.** The
 * owner's note of 28 Sep 2026, about the Plan node's edge then: *the connecting
 * line runs behind the node.* Every edge is drawn in one SVG below the node
 * layer, so an edge that crosses a card is simply invisible there — and the
 * loop passes the step between the two it joins, so the geometry is the only
 * thing that can say it went round rather than through.
 */
export const WithALoop: Story = {
  args: {
    nodes: steps,
    edges: [
      ...spine,
      { id: "tests>plan", source: "step:tests", target: "step:plan", kind: "returns", label: "up to 5 passes" },
    ],
    label: "The run",
    ...RUNS_DOWN,
  },
  play: async ({ canvas, canvasElement }) => {
    await waitFor(() => expect(canvas.getByLabelText("Write tests returns to Plan the change")).toBeInTheDocument());
    const loop = canvasElement.querySelector<SVGPathElement>('path[id="tests>plan"]')!;
    expect(loop).not.toBeNull();
    const card = canvas.getByRole("button", { name: "Implement, running" }).getBoundingClientRect();
    // An edge landing on a card's own side is on its border, so the box is
    // taken in by more than a line's width before anything is asked of it.
    const INSIDE = 6;
    const behind = pointsAlong(loop).filter(
      (at) =>
        at.x > card.left + INSIDE &&
        at.x < card.right - INSIDE &&
        at.y > card.top + INSIDE &&
        at.y < card.bottom - INSIDE,
    );
    expect(behind).toHaveLength(0);
  },
};

/**
 * Narrow opens on the step a person is on and its neighbours, rather than
 * fitting a whole run no one can read (`#1539`, revision of 22 Sep).
 *
 * **A `play`, because a still cannot say what the canvas was fitted to.** What
 * has to hold is that the step it opened on is legible — which is the whole
 * claim — and a card drawn at a tenth of its width still renders.
 */
export const OpensOnWhereYouAre: Story = {
  args: {
    nodes: steps,
    edges: spine,
    label: "The run",
    running: "step:implement",
    opensOn: [["step:plan", "step:implement", "step:tests"], ["step:implement"]],
    ...RUNS_DOWN,
  },
  play: async ({ canvas }) => {
    await waitFor(() => {
      const step = canvas.getByRole("button", { name: "Implement, running" });
      expect(step.getBoundingClientRect().width).toBeGreaterThan(160);
    });
  },
};

/**
 * The follow toggle, on the rail and in a group of its own under the view
 * group — the owner's drawing of 28 Sep 2026, which stacked the groups down
 * the canvas's left with air between them.
 *
 * **A `play`, because where the control is is the claim.** It is icon-only
 * now, so its name has to survive the glyph and say which state it is in; and
 * it is not in the view group, because a zoom acts once and this hands the
 * viewport over until it is pressed again.
 */
export const FollowsTheRun: Story = {
  args: {
    nodes: steps,
    edges: spine,
    label: "The run",
    running: "step:implement",
    following: false,
    onFollowing: () => undefined,
    ...RUNS_DOWN,
  },
  play: async ({ canvas }) => {
    const follows = canvas.getByRole("group", { name: "What the view follows" });
    await waitFor(() => expect(follows).toBeVisible());
    const stay = canvas.getByRole("button", { name: "Stay on the running step" });
    await expect(stay).toHaveAttribute("aria-pressed", "false");
    const view = canvas.getByRole("group", { name: "How you are looking at this" });
    await expect(view).not.toContainElement(stay);
    await expect(view.querySelectorAll("button")).toHaveLength(3);
  },
};
