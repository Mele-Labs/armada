import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fireEvent, fn, waitFor, within } from "storybook/test";

import { STUDIO_NODE_KIND } from "../StudioNode/StudioNode";
import { StudioWhiteboard, type StudioWhiteboardEdge, type StudioWhiteboardNode } from "./StudioWhiteboard";

const meta: Meta<typeof StudioWhiteboard> = {
  title: "Compositions/Studio whiteboard",
  component: StudioWhiteboard,
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

type Story = StoryObj<typeof StudioWhiteboard>;

/** One stretch of work: a board and a failing run read in, noted, asked about, written up and dispatched. */
const nodes: StudioWhiteboardNode[] = [
  { id: "link", position: { x: 0, y: 0 }, node: { kind: "link", title: "Studio proposal board", address: "https://miro.com/app/board/uXjVK" } },
  { id: "run", position: { x: 0, y: 260 }, node: { kind: "run", state: "running", title: "pnpm test", facts: ["test", "1m 12s"] } },
  { id: "note-legend", position: { x: 340, y: 0 }, node: { kind: "note", title: "The legend under the step bar is unreadable", facts: ["Board"] } },
  { id: "note-width", position: { x: 340, y: 260 }, node: { kind: "note", title: "The legend wraps at 720 wide", facts: ["Board"] } },
  { id: "contradiction", position: { x: 340, y: 520 }, node: { kind: "contradiction", state: "reported", title: "bridge.md and react.md disagree on the pulse", facts: ["2 sources"] } },
  { id: "finding-colours", position: { x: 680, y: -260 }, node: { kind: "finding", state: "gathering", title: "Where the legend's colours come from", facts: ["$0.12", "14 files read"] } },
  { id: "finding-file", position: { x: 680, y: 0 }, node: { kind: "finding", state: "frozen", title: "Fleet writes fleet.json once, at start", facts: ["$0.31"] } },
  { id: "cluster", position: { x: 680, y: 260 }, node: { kind: "cluster", title: "The Board's legend is illegible", facts: ["2 notes"] } },
  { id: "sketch", position: { x: 680, y: 520 }, node: { kind: "sketch", title: "", drawing: { boxes: [{ id: "b1", x: 0, y: 0, body: "The legend, redrawn" }], lines: [], strokes: [], pictures: [] } } },
  { id: "picture", position: { x: 1020, y: 520 }, node: { kind: "picture", title: "", frame: {} } },
  { id: "file", position: { x: 680, y: 780 }, node: { kind: "file", title: "packages/components/src/compositions/StatusLegend/StatusLegend.css", path: "packages/components/src/compositions/StatusLegend/StatusLegend.css" } },
  { id: "deferral", position: { x: 1020, y: -130 }, node: { kind: "deferral", state: "open", title: "Does the legend belong on the Board?", facts: ["blocks 1"] } },
  { id: "outline", position: { x: 1020, y: 260 }, node: { kind: "outline", state: "draft", title: "Legend, then width", facts: ["3 parts"] } },
  // The three kinds a forge address makes — #1394. Each is a record of
  // something already filed, drawn beside what a person worked out about it.
  { id: "forge-issue", position: { x: 0, y: 520 }, node: { kind: "issue", title: "The Board's legend is illegible", address: "https://example.invalid/armada/issues/1394", facts: ["#1394", "Open"] } },
  { id: "forge-pull", position: { x: 0, y: 780 }, node: { kind: "pull_request", title: "Dispatch an issue from its node", address: "https://example.invalid/armada/pull/1391", facts: ["#1391", "Merged"] } },
  { id: "forge-epic", position: { x: 340, y: 780 }, node: { kind: "epic", title: "Studio", address: "https://example.invalid/armada/milestone/17", facts: ["#17", "12 of 30 issues"] } },
  { id: "issue", position: { x: 1360, y: 260 }, node: { kind: "issue_draft", state: "draft", title: "The Board's legend is illegible" } },
  { id: "job", position: { x: 1700, y: 260 }, node: { kind: "job", state: "running", title: "The Board's legend is illegible", facts: ["j-3f2a", "bug"] } },
];

/** Every edge kind; each relation once proposed and once accepted. */
const edges: StudioWhiteboardEdge[] = [
  { id: "e1", source: "link", target: "note-legend", kind: "produced" },
  { id: "e2", source: "link", target: "contradiction", kind: "produced" },
  { id: "e-epic", source: "forge-epic", target: "forge-issue", kind: "produced" },
  { id: "e3", source: "run", target: "note-width", kind: "produced" },
  { id: "e4", source: "note-legend", target: "finding-file", kind: "produced" },
  { id: "e5", source: "note-legend", target: "cluster", kind: "produced" },
  { id: "e6", source: "note-width", target: "cluster", kind: "produced" },
  { id: "e7", source: "cluster", target: "outline", kind: "produced" },
  { id: "e8", source: "outline", target: "issue", kind: "produced" },
  { id: "e9", source: "issue", target: "job", kind: "produced" },
  { id: "e10", source: "contradiction", target: "sketch", kind: "produced" },
  { id: "r1", source: "note-legend", target: "note-width", kind: "same_as", proposed: false },
  { id: "r2", source: "sketch", target: "cluster", kind: "same_as", proposed: true, proposer: "Helm proposes" },
  { id: "r3", source: "deferral", target: "outline", kind: "blocks", proposed: false },
  { id: "r4", source: "sketch", target: "outline", kind: "blocks", proposed: true },
  { id: "r5", source: "finding-file", target: "deferral", kind: "answers", proposed: false },
  { id: "r6", source: "finding-colours", target: "deferral", kind: "answers", proposed: true },
];

function centre(element: Element) {
  const box = element.getBoundingClientRect();
  return { x: box.left + box.width / 2, y: box.top + box.height / 2, width: box.width };
}

export const EveryKind: Story = {
  args: { nodes, edges, onNodeMoved: fn(), onSelectionChange: fn(), onDecide: fn() },
  play: async ({ canvas, args, userEvent, step }) => {
    await step("one node of every kind, each named for a reader", async () => {
      for (const kind of Object.values(STUDIO_NODE_KIND)) {
        // A Picture and a Sketch have no title, and are named by their kind and any state.
        const named = await canvas.findAllByRole("group", { name: new RegExp(`^${kind}(: |, |$)`) });
        await expect(named.length).toBeGreaterThan(0);
      }
    });

    await step("each relation labelled, and a proposed one says so", async () => {
      for (const label of ["same as", "blocks", "answers"]) {
        await expect(canvas.getAllByText(label)).toHaveLength(2);
      }
      await expect(canvas.getAllByRole("group", { name: /, proposed, waiting on you$/ })).toHaveLength(3);
      await expect(canvas.getAllByRole("group", { name: / produced / })).toHaveLength(11);
    });

    await step("a proposed relation is answered on its own label, which names who proposed it", async () => {
      const proposal = canvas.getByRole("group", { name: /^Helm proposes: Sketch\s+same as / });
      await userEvent.click(within(proposal).getByRole("button", { name: /^Accept: / }));
      await expect(args.onDecide).toHaveBeenCalledWith("r2", true);
      // Where the record names nobody, the label says the bare fact.
      await expect(canvas.getAllByRole("group", { name: /^Proposed: / })).toHaveLength(2);
    });

    const note = canvas.getByRole("group", { name: /^Note: The legend under the step bar/ });
    await waitFor(() => expect(note).toBeVisible());

    await step("a node dragged to a new place moves there, and its neighbours do not", async () => {
      // Measured against a neighbour: a drag that missed the node pans the
      // whole board, which moves every node on screen by the same amount.
      const link = canvas.getByRole("group", { name: /^Link: / });
      const apart = () => ({ x: centre(note).x - centre(link).x, y: centre(note).y - centre(link).y });
      const before = centre(note);
      const gap = apart();
      const view = note.ownerDocument.defaultView ?? window;
      fireEvent.mouseDown(note, { clientX: before.x, clientY: before.y, button: 0, view });
      for (const [dx, dy] of [[10, 10], [60, 40], [120, 80]] as const) {
        fireEvent.mouseMove(view, { clientX: before.x + dx, clientY: before.y + dy, view });
      }
      fireEvent.mouseUp(view, { clientX: before.x + 120, clientY: before.y + 80, view });
      await waitFor(() => expect(apart().x).toBeGreaterThan(gap.x + 60));
      await expect(apart().y).toBeGreaterThan(gap.y + 30);
      await expect(args.onNodeMoved).toHaveBeenCalledWith("note-legend", expect.anything());
    });

    await step("a node is selected and moved by keyboard", async () => {
      const job = canvas.getByRole("group", { name: /^Job: / });
      job.focus();
      await userEvent.keyboard("{Enter}");
      await expect(args.onSelectionChange).toHaveBeenLastCalledWith(["job"]);
      const issue = canvas.getByRole("group", { name: /^Issue draft: / });
      const before = centre(job).y - centre(issue).y;
      await userEvent.keyboard("{Shift>}{ArrowDown}{/Shift}");
      await waitFor(() => expect(centre(job).y - centre(issue).y).toBeGreaterThan(before));
      await expect(args.onNodeMoved).toHaveBeenCalledWith("job", expect.anything());
    });

    await step("zoom in draws every node larger", async () => {
      const before = centre(note).width;
      await userEvent.click(canvas.getByRole("button", { name: "Zoom in" }));
      await waitFor(() => expect(centre(note).width).toBeGreaterThan(before));
    });
  },
};

/** A Studio reopened read-only: nothing drags, by pointer or by arrow key, and nothing is saved. */
export const ReadOnly: Story = {
  args: { nodes, edges, readOnly: true, onNodeMoved: fn(), onSelectionChange: fn(), onDecide: fn() },
  play: async ({ canvas, args, userEvent, step }) => {
    const note = canvas.getByRole("group", { name: /^Note: The legend under the step bar/ });
    await waitFor(() => expect(note).toBeVisible());
    const link = canvas.getByRole("group", { name: /^Link: / });
    const apart = () => centre(note).x - centre(link).x;

    await step("a drag moves nothing and reports nothing", async () => {
      const gap = apart();
      const before = centre(note);
      const view = note.ownerDocument.defaultView ?? window;
      fireEvent.mouseDown(note, { clientX: before.x, clientY: before.y, button: 0, view });
      for (const [dx, dy] of [[10, 10], [60, 40], [120, 80]] as const) {
        fireEvent.mouseMove(view, { clientX: before.x + dx, clientY: before.y + dy, view });
      }
      fireEvent.mouseUp(view, { clientX: before.x + 120, clientY: before.y + 80, view });
      await expect(Math.abs(apart() - gap)).toBeLessThan(1);
      await expect(args.onNodeMoved).not.toHaveBeenCalled();
    });

    await step("a node still selects, and an arrow key moves nothing", async () => {
      const job = canvas.getByRole("group", { name: /^Job: / });
      job.focus();
      await userEvent.keyboard("{Enter}");
      await expect(args.onSelectionChange).toHaveBeenLastCalledWith(["job"]);
      await userEvent.keyboard("{Shift>}{ArrowDown}{/Shift}");
      await expect(args.onNodeMoved).not.toHaveBeenCalled();
    });

    await step("a proposed relation says how to answer it, and offers no answer", async () => {
      await expect(canvas.getAllByText("Continue to accept or reject.")).toHaveLength(3);
      await expect(canvas.queryByRole("button", { name: /^Accept: / })).toBeNull();
    });
  },
};

/**
 * Opened *at* a node rather than at nothing — a Job's detail reaching the
 * Studio that dispatched it, #1362.
 *
 * **A `play`, because a still cannot tell the two apart.** What has to hold is
 * that the board reports the selection it was given without anyone clicking,
 * and that a person's own click then wins: a prop that re-asserted itself
 * every render would take the board back off them.
 */
export const OpenedAtANode: Story = {
  args: { nodes, edges, pick: "job", onNodeMoved: fn(), onSelectionChange: fn() },
  play: async ({ canvas, args, userEvent, step }) => {
    const job = canvas.getByRole("group", { name: /^Job: / });
    await waitFor(() => expect(job).toBeVisible());

    await step("the node it was opened at is selected, with nothing pressed", async () => {
      await waitFor(() => expect(args.onSelectionChange).toHaveBeenLastCalledWith(["job"]));
    });

    await step("a person's own pick takes it from there", async () => {
      const note = canvas.getByRole("group", { name: /^Note: The legend under the step bar/ });
      note.focus();
      await userEvent.keyboard("{Enter}");
      await waitFor(() => expect(args.onSelectionChange).toHaveBeenLastCalledWith(["note-legend"]));
    });
  },
};
