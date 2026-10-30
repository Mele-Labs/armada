import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import { expect, waitFor } from "storybook/test";

import { RevisionMenu } from "./RevisionMenu";

const meta: Meta<typeof RevisionMenu> = {
  title: "Compositions/Revision menu",
  component: RevisionMenu,
};
export default meta;

type Story = StoryObj<typeof RevisionMenu>;

function Held({ count, start }: { count: number; start: number }) {
  const [revision, setRevision] = useState(start);
  return <RevisionMenu count={count} revision={revision} onPick={setRevision} />;
}

/** It opens on the newest. Its menu lists every revision newest first, and a pick is what the trigger then names. */
export const OnTheNewest: Story = {
  render: () => <Held count={3} start={3} />,
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Revision 3" }));
    await waitFor(() => expect(canvas.getByRole("menu")).toBeVisible());
    await expect(canvas.getAllByRole("menuitem").map((item) => item.textContent)).toEqual(["Revision 3", "Revision 2", "Revision 1"]);
    await expect(canvas.getByRole("menuitem", { name: "Revision 3" })).toHaveAttribute("aria-current", "true");
    await userEvent.click(canvas.getByRole("menuitem", { name: "Revision 1" }));
    await expect(canvas.queryByRole("menu")).toBeNull();
    await expect(canvas.getByRole("button", { name: "Revision 1" })).toBeInTheDocument();
  },
};

/** An earlier revision picked: the trigger names it, and the menu checks it. */
export const OnAnEarlierOne: Story = { render: () => <Held count={3} start={2} /> };
