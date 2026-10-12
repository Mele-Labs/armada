import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";

import { CommandPalette } from "./CommandPalette";

const meta: Meta<typeof CommandPalette> = {
  title: "Primitives/CommandPalette",
  component: CommandPalette,
};
export default meta;

type Story = StoryObj<typeof CommandPalette>;

/**
 * **Autopilot is the third kind, on a picked repository** (#2092). The row offers the words as an
 * Autopilot's sentence, which opens the Cockpit's start form rather than starting anything, and ⇥ goes
 * on round to a Job.
 */
export const AutopilotIsTheThirdKind: Story = {
  args: {
    open: true,
    sections: [],
    entries: [],
    searched: "",
    defaultQuery: "work through the open bugs, two at a time, until 7am",
    dispatch: { kind: "autopilot", kinds: ["job", "session", "autopilot"], onDispatch: fn(), onKind: fn() },
  },
  play: async ({ args, canvas, userEvent }) => {
    await expect(canvas.getByRole("option", { name: /^Start Autopilot with “work through the open bugs, two at a time, until 7am”/ })).toBeVisible();
    await expect(canvas.getByText("Job / Session / Autopilot")).toBeInTheDocument();
    await userEvent.keyboard("{Tab}");
    await expect(args.dispatch!.onKind).toHaveBeenCalledWith("job");
    await userEvent.keyboard("{Enter}");
    await expect(args.dispatch!.onDispatch).toHaveBeenCalledWith("work through the open bugs, two at a time, until 7am", "autopilot");
  },
};
