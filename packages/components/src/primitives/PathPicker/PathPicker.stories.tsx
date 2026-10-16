import { useState } from "react";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, userEvent, within } from "storybook/test";
import { PathPicker } from "./PathPicker";

/**
 * The folder an "always" answer on a session's card is for. It opens on the whole path, the
 * narrowest choice, and a press on a folder above widens it.
 */
const meta: Meta<typeof PathPicker> = {
  title: "Primitives/Path picker",
  component: PathPicker,
};
export default meta;

const PATH = "/Users/a/Documents/my/folder/notes.md";

function Picking({ opens }: { opens: string }) {
  const [chosen, setChosen] = useState(opens);
  return (
    <div style={{ display: "grid", gap: "var(--space-2)" }}>
      <PathPicker path={PATH} chosen={chosen} onChoose={setChosen} label="Folder" />
      <output aria-label="Chosen">{chosen}</output>
    </div>
  );
}

/** The whole path chosen, which is where a card opens. */
export const WholePath: StoryObj = {
  render: () => <Picking opens={PATH} />,
};

/** A press on Documents chooses it and everything under it. */
export const AFolderAbove: StoryObj = {
  name: "A folder above",
  render: () => <Picking opens={PATH} />,
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByRole("button", { name: "Documents" }));
    await expect(canvas.getByRole("button", { name: "Documents" })).toHaveAttribute("aria-pressed", "true");
    await expect(canvas.getByRole("button", { name: "notes.md" })).toHaveAttribute("aria-pressed", "false");
    await expect(canvas.getByRole("status", { name: "Chosen" })).toHaveTextContent("/Users/a/Documents");
  },
};
