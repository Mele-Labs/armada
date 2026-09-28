import type { Meta, StoryObj } from "@storybook/react-vite";
import { Handle, type Edge, type Node, type NodeProps } from "@xyflow/react";
import { expect, waitFor } from "storybook/test";

import { SquarePlus, Undo2 } from "lucide-react";

import { Card, CardContent, CardTitle } from "../../primitives/Card/Card";
import { GRAPH_CANVAS_SIDES, GraphCanvas } from "./GraphCanvas";
import { GraphCanvasRailGroup } from "./GraphCanvasRail";

/**
 * The graph surface on its own, with three plain cards on it — what is left
 * when neither a Studio's node nor a workflow's step is drawn. The two real
 * surfaces are *Studio whiteboard* and *Workflow canvas*.
 */
const meta: Meta<typeof GraphCanvas> = {
  title: "Compositions/Graph canvas",
  component: GraphCanvas,
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

type Card = Node<{ title: string }, "card">;
type Line = Edge<Record<string, never>, "default">;

function CardView({ data, selected }: NodeProps<Card>) {
  return (
    <>
      {GRAPH_CANVAS_SIDES.map((side) => (
        <Handle key={`t-${side}`} id={`t-${side}`} type="target" position={side} isConnectable={false} />
      ))}
      <Card role="group" aria-label={data.title} aria-current={selected || undefined}>
        <CardContent>
          <CardTitle>{data.title}</CardTitle>
        </CardContent>
      </Card>
      {GRAPH_CANVAS_SIDES.map((side) => (
        <Handle key={`s-${side}`} id={`s-${side}`} type="source" position={side} isConnectable={false} />
      ))}
    </>
  );
}

const NODE_TYPES = { card: CardView };
const EDGE_TYPES = {};

const nodes: Card[] = [
  { id: "a", type: "card", position: { x: 0, y: 0 }, data: { title: "One" } },
  { id: "b", type: "card", position: { x: 260, y: 0 }, data: { title: "Two" } },
  { id: "c", type: "card", position: { x: 520, y: 0 }, data: { title: "Three" } },
];

const edges: Line[] = [
  { id: "a-b", source: "a", target: "b", sourceHandle: "s-right", targetHandle: "t-left" },
  { id: "b-c", source: "b", target: "c", sourceHandle: "s-right", targetHandle: "t-left" },
];

const args = { surface: "armada-graph-canvas-demo", label: "Three cards", nodes, edges, nodeTypes: NODE_TYPES, edgeTypes: EDGE_TYPES };

type Story = StoryObj<typeof GraphCanvas<Card, Line>>;

/**
 * The canvas with nothing on its rail but the view group — every canvas carries
 * that much, and a surface with nothing to place carries only that.
 *
 * **A `play`, because what a reader hears is the half a rendering cannot
 * show.** Every button on the rail is icon-only: `−` and `+` carry no
 * accessible name of their own and Fit is a glyph now, so without the names
 * below the group is read out as punctuation and a shape.
 */
export const ViewOnly: Story = {
  args,
  play: async ({ canvas }) => {
    await waitFor(() => expect(canvas.getByRole("button", { name: "Zoom in" })).toBeVisible());
    await expect(canvas.getByRole("button", { name: "Zoom in" })).toHaveTextContent("+");
    await expect(canvas.getByRole("button", { name: "Zoom out" })).toHaveTextContent("−");
    await expect(canvas.getByRole("button", { name: "Fit" })).toBeVisible();
    await expect(canvas.getByRole("group", { name: "How you are looking at this" })).toBeVisible();
  },
};

/**
 * A surface with tools of its own: its group sits above the view group, with a
 * gap between the two — the owner's own drawing of 28 Sep 2026.
 */
export const WithTools: Story = {
  args: {
    ...args,
    rail: (
      <GraphCanvasRailGroup
        label="What you can put on this"
        acts={[
          { id: "add", name: "Add a box", icon: SquarePlus, onPress: () => undefined },
          { id: "undo", name: "Undo", icon: Undo2, disabled: true, why: "Nothing to take back.", onPress: () => undefined },
        ]}
      />
    ),
  },
};
