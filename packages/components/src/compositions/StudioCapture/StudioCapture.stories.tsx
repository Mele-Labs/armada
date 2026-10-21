import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import { expect, fn, userEvent } from "storybook/test";

import { ACTION } from "../../generated/actions";
import { StudioCapture, type CaptureBox, type StudioCaptureProps } from "./StudioCapture";

/**
 * The layer over a window, so each story draws a box in the canvas the way the
 * app draws one over a control. The binding is the registry's own.
 */
const BINDING = [...(ACTION.capture_note?.shortcut ?? "")];

const meta: Meta<typeof StudioCapture> = {
  title: "Compositions/Studio capture",
  component: StudioCapture,
  args: {
    note: "",
    onNote: fn(),
    onSave: fn(),
    onCancel: fn(),
    aim: "Stale counts",
    aimed: true,
    binding: BINDING,
  },
};
export default meta;

type Story = StoryObj<typeof StudioCapture>;

const CHIP = { x: 120, y: 120, width: 96, height: 28 };

/** Pointing: the element under the pointer is outlined and named. */
export const Pointing: Story = {
  args: { hovered: { box: CHIP, named: "FilterChip" } },
};

/** Held: the card sits against the element, and the note is written in it. */
export const Writing: Story = {
  args: {
    held: { box: CHIP, chain: "FilterChip ← Board · button.armada-chip" },
    note: "The chip keeps its count after the filter is cleared",
  },
};

/** Out to Fleet. The card waits rather than taking a second press. */
export const Capturing: Story = {
  args: { ...Writing.args, saving: true },
};

/** Fleet did not take it, and the card says so without losing what was typed. */
export const Refused: Story = {
  args: { ...Writing.args, refused: "Fleet is not running, so nothing was captured." },
};

/**
 * No Studio is open, so there is nothing to capture onto and the bar says so
 * rather than offering a Capture that would fail.
 */
export const NothingAimedAt: Story = {
  args: {
    held: { box: CHIP, chain: "FilterChip ← Board · button.armada-chip" },
    note: "The chip keeps its count",
    aim: "No Studio open — open one on Studios to capture onto it",
    aimed: false,
  },
};

/**
 * **Capture is refused until something is said, and until a Studio is aimed
 * at.** A blank note would be a pin with nothing on it, and one with nowhere to
 * land would fail after the person had typed it — neither is a press to offer.
 * Escape closes the card without capturing.
 */
export const WhatTheCardRefuses: Story = {
  render: function Draft(args: StudioCaptureProps) {
    const [note, setNote] = useState("");
    return <StudioCapture {...args} note={note} onNote={setNote} />;
  },
  args: { held: { box: CHIP, chain: "FilterChip ← Board · button.armada-chip" } },
  play: async ({ args, canvas }) => {
    const capture = canvas.getByRole("button", { name: "Capture" });
    await expect(capture).toBeDisabled();

    await userEvent.type(canvas.getByLabelText("Note"), "   ");
    await expect(capture).toBeDisabled();

    await userEvent.type(canvas.getByLabelText("Note"), "the count is stale");
    await expect(capture).toBeEnabled();

    await userEvent.keyboard("{Escape}");
    await expect(args.onCancel).toHaveBeenCalled();
    await expect(args.onSave).not.toHaveBeenCalled();
  },
};

/**
 * **The cursor is in the note once the card opens.** The card is hidden until
 * it is placed, and a field cannot take focus while hidden, so the note is
 * focused once the card is in place rather than as it mounts.
 */
export const HoldingFocusesTheNote: Story = {
  render: function Hold(args: StudioCaptureProps) {
    const [held, setHeld] = useState<StudioCaptureProps["held"]>(undefined);
    return (
      <>
        <button type="button" onClick={() => setHeld({ box: CHIP, chain: "FilterChip ← Board · button.armada-chip" })}>
          Hold the chip
        </button>
        <StudioCapture {...args} held={held} />
      </>
    );
  },
  play: async ({ canvas }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Hold the chip" }));
    await expect(canvas.getByLabelText("Note")).toHaveFocus();
  },
};

/**
 * **The card lands inside the box it is drawn in, not the screen.** A walk
 * window draws the layer over its page column, so a card sized and flipped
 * against the viewport was cut at the column's top, bottom or right edge. The
 * transform makes this box the layer's containing block, as the column is.
 */
function Box({ children }: { children: React.ReactNode }) {
  return (
    <div
      role="region"
      aria-label="Page"
      style={{
        position: "relative",
        overflow: "hidden",
        transform: "translateZ(0)",
        width: "calc(var(--w-tooltip) + var(--space-12) * 2)",
        height: "calc(var(--space-12) * 6)",
        background: "var(--surface-canvas)",
      }}
    >
      {children}
    </div>
  );
}

const ROOM = { width: 436, height: 288 };
const AT_EDGE = (box: CaptureBox): Partial<StudioCaptureProps> => ({
  held: { box, chain: "FilterChip ← Board · button.armada-chip" },
  note: "The chip keeps its count after the filter is cleared",
});

async function expectCardInside(canvas: { getByRole: (role: string, options?: { name: string }) => HTMLElement }) {
  const page = canvas.getByRole("region", { name: "Page" }).getBoundingClientRect();
  const card = canvas.getByRole("dialog", { name: "Capture a note" }).getBoundingClientRect();
  await expect(card.top).toBeGreaterThanOrEqual(page.top);
  await expect(card.bottom).toBeLessThanOrEqual(page.bottom);
  await expect(card.left).toBeGreaterThanOrEqual(page.left);
  await expect(card.right).toBeLessThanOrEqual(page.right);
}

/** Against the top edge: below it, where there is room. */
export const AtTheTopEdge: Story = {
  decorators: [(Story) => <Box><Story /></Box>],
  args: AT_EDGE({ x: 120, y: 0, width: 96, height: 28 }),
  play: ({ canvas }) => expectCardInside(canvas),
};

/** Against the bottom edge: above it, since nothing is left below. */
export const AtTheBottomEdge: Story = {
  decorators: [(Story) => <Box><Story /></Box>],
  args: AT_EDGE({ x: 120, y: ROOM.height - 28, width: 96, height: 28 }),
  play: ({ canvas }) => expectCardInside(canvas),
};

/** Against the right edge: pulled left, so the card is not cut at the side. */
export const AtTheRightEdge: Story = {
  decorators: [(Story) => <Box><Story /></Box>],
  args: AT_EDGE({ x: ROOM.width - 96, y: 130, width: 96, height: 28 }),
  play: ({ canvas }) => expectCardInside(canvas),
};
