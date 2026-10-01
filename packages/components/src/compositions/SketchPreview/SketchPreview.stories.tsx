import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect } from "storybook/test";

import { SketchPreview } from "./SketchPreview";

/**
 * A pad's drawing, fitted to the box it is given. Drawn here in the card's
 * picture plate shape, which is where a Studio's Sketch node puts it.
 */
const meta: Meta<typeof SketchPreview> = {
  title: "Compositions/Sketch preview",
  component: SketchPreview,
  decorators: [
    (Story) => (
      <div className="armada-studio-node__frame" style={{ width: "var(--w-studio-node)" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof SketchPreview>;

/**
 * A stand-in screenshot. **A named colour, and not a design value**: what is
 * inside a picture is a photograph of a window, which no token applies to.
 */
const SHOT = `data:image/svg+xml;utf8,${encodeURIComponent(
  `<svg xmlns="http://www.w3.org/2000/svg" width="640" height="400"><rect width="640" height="400" fill="slategray"/></svg>`,
)}`;

const BOXES = [
  { id: "b1", x: 0, y: 0, body: "The rail" },
  { id: "b2", x: 360, y: 40, body: "The board" },
];

/** Two boxes joined, a line drawn by hand, and a pasted picture joined to a box. */
export const Drawn: Story = {
  args: {
    label: "The sketch on this Studio",
    boxes: BOXES,
    lines: [
      { id: "b1-b2", from: "b1", to: "b2" },
      { id: "b2-p1", from: "b2", to: "p1" },
    ],
    strokes: [
      {
        id: "s1",
        points: [
          { x: 20, y: 140 },
          { x: 80, y: 180 },
          { x: 160, y: 150 },
        ],
      },
    ],
    pictures: [{ id: "p1", x: 360, y: 200, width: 240, height: 150, src: SHOT }],
  },
  play: async ({ canvas }) => {
    // One picture, named for a reader who cannot see it, holding the words.
    await expect(canvas.getByRole("img", { name: "The sketch on this Studio" })).toBeVisible();
    await expect(canvas.getByText("The rail")).toBeInTheDocument();
    await expect(canvas.getByText("The board")).toBeInTheDocument();
  },
};

/** A picture whose bytes have not been read yet takes its space, and nothing moves when they land. */
export const PictureBeingRead: Story = {
  args: {
    label: "The sketch on this Studio",
    boxes: [],
    lines: [],
    strokes: [],
    pictures: [{ id: "p1", x: 0, y: 0, width: 240, height: 150 }],
  },
};

/** A Sketch written as text before 1 Oct 2026: one box holding it. */
export const OneBox: Story = {
  args: {
    label: "The sketch on this Studio",
    boxes: [{ id: "b1", x: 0, y: 0, body: "Legend on its own row\nunder the step bar" }],
    lines: [],
    strokes: [],
    pictures: [],
  },
};
