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
    // No `revisions`, no menu: a plan's sketch or one Fleet kept nothing for.
    await expect(canvas.queryByRole("button", { name: /^Revision/ })).toBeNull();
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

/** The first drawing: the caller alone. */
const FIRST: Scene = { nodes: [SCENE.nodes[0]!], edges: [] };
/** The second adds the writer, not yet joined. */
const SECOND: Scene = { nodes: SCENE.nodes, edges: [] };

/**
 * Drawn three times under one title: it opens on the newest, drawn against the one before. An earlier revision is drawn
 * against the one before it, the first plain, and Approve still answers the sketch.
 */
export const Revisions: Story = {
  args: { ...ARGS, onAnswer: fn(), revisions: [FIRST, SECOND, SCENE] },
  play: async ({ canvas, args }) => {
    await expect(canvas.getByRole("button", { name: "Revision 3" })).toBeInTheDocument();
    await expect(canvas.getByText("Against Revision 2")).toBeInTheDocument();
    await userEvent.click(canvas.getByRole("button", { name: "Revision 3" }));
    await userEvent.click(canvas.getByRole("menuitem", { name: "Revision 2" }));
    await expect(canvas.getByText("Against Revision 1")).toBeInTheDocument();
    await expect(canvas.getByRole("group", { name: "Added, Writer" })).toBeInTheDocument();
    await userEvent.click(canvas.getByRole("button", { name: "Revision 2" }));
    await userEvent.click(canvas.getByRole("menuitem", { name: "Revision 1" }));
    await expect(canvas.queryByText(/^Against/)).toBeNull();
    await expect(canvas.queryByRole("group", { name: "Read as" })).toBeNull();
    await expect(canvas.getByRole("group", { name: /^Caller$/ })).toBeInTheDocument();
    await userEvent.click(canvas.getByRole("button", { name: "Approve" }));
    await expect(args.onAnswer).toHaveBeenCalledWith("k1", { choice: 0 });
  },
};

/** Drawn once and kept: the menu is there, as a document's is, and the drawing has nothing to be read against. */
export const OneRevision: Story = {
  args: { ...ARGS, revisions: [SCENE] },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("button", { name: "Revision 1" })).toBeInTheDocument();
    await expect(canvas.queryByText(/^Against/)).toBeNull();
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

/** Withdraw shows only where something takes the sketch back, and hands over the sketch's id. */
export const Withdraws: Story = {
  args: { ...ARGS, onWithdraw: fn() },
  play: async ({ canvas, args }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Withdraw" }));
    await expect(args.onWithdraw).toHaveBeenCalledWith("k1");
  },
};

/** With nothing to take it back, the card offers Approve and Send back only. */
export const NoWithdrawWithoutOne: Story = {
  args: ARGS,
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("button", { name: "Withdraw" })).toBeNull();
  },
};
