import type { ReactElement, ReactNode } from "react";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, waitFor, within } from "storybook/test";

import { forgetKept, writeKept } from "../../keep";
import type { StepFixes } from "./StepsPanel";
import { StepsPanel } from "./StepsPanel";

const STEPS = [
  "Name it Armada",
  "Any homepage URL and callback URL",
  "Tick Enable Device Flow",
  "Upload a logo",
  "Press Register application",
  "Copy the Client ID and send it here",
];
const ADDRESS = "https://auth.example.com/apps/new";
const NAME = "Register the OAuth app";

/** Each story keeps its ticks, corrections and draft under a key of its own, so none leaks into the next. */
const keyOf = (story: string) => `story:steps:${story}`;
const forget = (keep: string) => {
  forgetKept(`${keep}:ticked`);
  forgetKept(`${keep}:fixes`);
  forgetKept(`${keep}:reply`);
};

/** The panel at the width the thread's side column gives it. */
function Beside({ children }: { children: ReactNode }): ReactElement {
  return <div style={{ width: "var(--w-sheet)", background: "var(--bg-sunken)", padding: "var(--space-4)" }}>{children}</div>;
}

const meta: Meta<typeof StepsPanel> = {
  title: "Compositions/Steps panel",
  component: StepsPanel,
  args: { name: NAME, address: ADDRESS, steps: STEPS, keep: keyOf("fresh"), onOpenAddress: fn(), onOpenInBrowser: fn(), onSend: fn() },
  render: (args) => (
    <Beside>
      <StepsPanel {...args} />
    </Beside>
  ),
};
export default meta;

type Story = StoryObj<typeof StepsPanel>;

/** Nothing ticked, nothing corrected, nothing typed: the key reads Done and answers with that word. */
export const Fresh: Story = {
  beforeEach: () => forget(keyOf("fresh")),
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getAllByRole("checkbox")).toHaveLength(STEPS.length);
    await expect(canvas.getByRole("button", { name: ADDRESS })).toBeVisible();
    await expect(canvas.queryByRole("button", { name: "Send" })).toBeNull();
    await userEvent.click(canvas.getByRole("button", { name: "Done" }));
    await expect(args.onSend).toHaveBeenCalledWith("Done");
  },
};

/** The ticks are the person's own place-keeping: the key stays Done, and typing turns it into Send. */
export const SomeTicked: Story = {
  args: { keep: keyOf("ticked") },
  beforeEach: () => {
    forget(keyOf("ticked"));
    writeKept(`${keyOf("ticked")}:ticked`, [0, 2]);
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("checkbox", { name: STEPS[0] })).toBeChecked();
    await expect(canvas.getByRole("checkbox", { name: STEPS[1] })).not.toBeChecked();
    await expect(canvas.getByRole("checkbox", { name: STEPS[2] })).toBeChecked();
    await expect(canvas.getByRole("button", { name: "Done" })).toBeVisible();
    await userEvent.type(canvas.getByRole("textbox", { name: `Reply to ${NAME}` }), "Registered");
    await expect(canvas.getByRole("button", { name: "Send" })).toBeVisible();
  },
};

const FIXES: StepFixes = { edited: { 1: "Homepage and callback both http://127.0.0.1:8470" }, dropped: [3], added: ["Leave the Client secret alone"] };

/** One step reworded, one dropped (struck through, kept, and brought back by its own key) and one added at the end. */
export const Corrected: Story = {
  args: { keep: keyOf("corrected") },
  beforeEach: () => {
    forget(keyOf("corrected"));
    writeKept(`${keyOf("corrected")}:fixes`, FIXES);
  },
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("checkbox", { name: FIXES.edited[1] })).toBeInTheDocument();
    await expect(canvas.getByRole("img", { name: `Step 2 was ${STEPS[1]}` })).toBeVisible();
    await expect(canvas.getByRole("button", { name: "Bring back step 4" })).toBeVisible();
    await expect(canvas.getByRole("checkbox", { name: FIXES.added[0] })).toBeInTheDocument();
    await userEvent.click(canvas.getByRole("button", { name: "Done" }));
    await expect(args.onSend).toHaveBeenCalledWith(
      [
        "Done",
        [
          `Step 2 changed: "${STEPS[1]}" → "${FIXES.edited[1]}"`,
          `Step 4 dropped: "${STEPS[3]}"`,
          `Added after step ${STEPS.length}: "${FIXES.added[0]}"`,
        ].join("\n"),
      ].join("\n\n"),
    );
  },
};

/** The steps are not about a page, so the key at the top is not drawn. */
export const NoAddress: Story = {
  args: { address: undefined, keep: keyOf("no-address") },
  beforeEach: () => forget(keyOf("no-address")),
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getAllByRole("checkbox")).toHaveLength(STEPS.length);
    await expect(canvas.queryByRole("button", { name: "Open in browser" })).toBeNull();
  },
};

const RUNS = ["Run `git fetch origin`", "Run `make broken`"];

/** A command with a runner gets Run: a clean exit ticks its step, and a failed one tells the Job with the exit code and what it printed. */
export const RunsCommands: Story = {
  args: {
    keep: keyOf("runs"),
    address: undefined,
    steps: RUNS,
    onFailed: fn(),
    runner: (command, on) => {
      setTimeout(() => {
        if (command === "make broken") on({ kind: "out", text: "make: *** No rule\r\n" });
        on({ kind: "end", code: command === "make broken" ? 2 : 0 });
      }, 0);
      return { stop: () => {} };
    },
  },
  beforeEach: () => forget(keyOf("runs")),
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement);
    const [fetch, broken] = canvas.getAllByRole("button", { name: "Run" });
    await userEvent.click(fetch!);
    await waitFor(() => expect(canvas.getByRole("checkbox", { name: "Run git fetch origin" })).toBeChecked());
    await userEvent.click(broken!);
    await waitFor(() => expect(args.onFailed).toHaveBeenCalledWith("Step 2: `make broken` failed, exit code 2.\n\nmake: *** No rule"));
    await expect(canvas.getByRole("checkbox", { name: "Run make broken" })).not.toBeChecked();
  },
};
