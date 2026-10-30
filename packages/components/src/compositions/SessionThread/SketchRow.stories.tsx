import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent } from "storybook/test";

import { SketchPin } from "./SketchRow";
import type { Scene } from "../SketchScene/scene";

/** A sketch waiting on the person, pinned above the composer: it folds to its header line, opens in the panel, and is approved or sent back from where it stands. */
const meta: Meta<typeof SketchPin> = {
  title: "Compositions/Session thread/Sketch pin",
  component: SketchPin,
  decorators: [
    (Story) => (
      <div style={{ inlineSize: "calc(var(--space-12) * 8)" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof SketchPin>;

const SCENE: Scene = {
  nodes: [
    { id: "a", kind: "box", x: 0, y: 40, title: "Caller", body: "append(batch)", icon: "terminal", step: 1 },
    { id: "b", kind: "box", x: 300, y: 40, title: "Writer", body: "write(batch)", icon: "hard-drive", step: 2 },
  ],
  edges: [{ id: "a-b", from: "a", to: "b", label: "append" }],
};

const ARGS = { sketch: "k1", title: "Store clock", scene: SCENE, keep: "sketch-pin-story", onAnswer: fn(), onOpen: fn() };

/** Fold hides the drawing and keeps Approve; Unfold brings the drawing back. */
export const Folds: Story = {
  args: ARGS,
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("button", { name: "Approve" })).toBeInTheDocument();
    await expect(canvas.getByRole("button", { name: "Fold" })).toHaveAttribute("aria-expanded", "true");
    await expect(canvas.getByRole("group", { name: "Sketch" })).toBeInTheDocument();
    await userEvent.click(canvas.getByRole("button", { name: "Fold" }));
    await expect(canvas.queryByRole("group", { name: "Sketch" })).toBeNull();
    await expect(canvas.getByRole("button", { name: "Approve" })).toBeInTheDocument();
    await userEvent.click(canvas.getByRole("button", { name: "Unfold" }));
    await expect(canvas.getByRole("group", { name: "Sketch" })).toBeInTheDocument();
    await expect(canvas.getByRole("button", { name: "Fold" })).toBeInTheDocument();
  },
};

/** Open in the panel hands the sketch to the panel, and Approve answers it approved. */
export const OpensAndApproves: Story = {
  args: { ...ARGS, onAnswer: fn(), onOpen: fn() },
  play: async ({ canvas, args }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Open in the panel" }));
    await expect(args.onOpen).toHaveBeenCalledWith("k1");
    await userEvent.click(canvas.getByRole("button", { name: "Approve" }));
    await expect(args.onAnswer).toHaveBeenCalledWith("k1", { choice: 0 });
  },
};
