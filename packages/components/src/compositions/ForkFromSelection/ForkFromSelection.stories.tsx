import { useRef } from "react";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, waitFor, within } from "storybook/test";

import { FORK_ROW, ForkFromSelection } from "./ForkFromSelection";

/** The bar over words selected in a message, whose one act forks the Session from it. */
const meta: Meta<typeof ForkFromSelection> = {
  title: "Compositions/Fork from selection",
  component: ForkFromSelection,
};
export default meta;

type Story = StoryObj<typeof ForkFromSelection>;

function Thread({ onFork }: { onFork: (pick: { rowId: string; quote: string }) => void }) {
  const scope = useRef<HTMLDivElement>(null);
  return (
    <div ref={scope}>
      <p {...{ [FORK_ROW]: "row-1" }}>The reader splits on newlines, so a line longer than the buffer is dropped.</p>
      <ForkFromSelection scope={scope} onFork={onFork} />
    </div>
  );
}

export const Selected: Story = {
  args: { onFork: fn() },
  render: (args) => <Thread onFork={args.onFork} />,
  play: async ({ args, canvasElement }) => {
    const words = within(canvasElement).getByText(/The reader splits/);
    window.getSelection()?.selectAllChildren(words);
    const body = within(document.body);
    await waitFor(() => expect(body.getByRole("toolbar", { name: "Selection" })).toBeVisible());
    await userEvent.click(body.getByRole("button", { name: "Fork from here" }));
    await expect(args.onFork).toHaveBeenCalledWith({
      rowId: "row-1",
      quote: "> The reader splits on newlines, so a line longer than the buffer is dropped.\n\n",
    });
  },
};
