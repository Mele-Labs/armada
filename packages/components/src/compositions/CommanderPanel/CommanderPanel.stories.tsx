import { useState } from "react";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent } from "storybook/test";
import { ArrowUpToLine } from "lucide-react";

import { CommanderPanel, type CommanderPanelProps } from "./CommanderPanel";
import type { SessionThreadRow } from "../SessionThread/SessionThread";

/**
 * A Job's Commander, over its canvas where the lead was: a band that carries its state, its thread
 * and the one box that talks to it. A Commander that has done nothing draws the band and the box.
 */
const meta: Meta<typeof CommanderPanel> = {
  title: "Compositions/Commander panel",
  component: CommanderPanel,
  args: { jobId: "job-a", state: "idle", rows: [], onSend: fn() },
  decorators: [
    (Story) => (
      <div style={{ display: "flex", height: "calc(var(--space-12) * 8)", width: "calc(var(--space-12) * 14)" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof CommanderPanel>;

const WOKEN: SessionThreadRow[] = [
  { id: "c1", at: "14:20:02", kind: "woke", why: "Check store failed" },
  { id: "c2", at: "14:20:05", kind: "tool", text: "Read crates/store/tests/flaky.rs" },
  { id: "c3", at: "14:20:12", kind: "act", icon: ArrowUpToLine, said: "Raised cap", text: "Turn cap 40 → 60" },
  { id: "c4", at: "14:20:15", kind: "message", from: "agent", text: "The Drone ran out of turns on the third retry." },
];

/** Nothing done: the band and the box, with no sentence standing in for a thread. */
export const Empty: Story = {
  args: { onHide: fn() },
  play: async ({ canvas, args }) => {
    await expect(canvas.getByRole("region", { name: "Commander" })).toBeVisible();
    await expect(canvas.getByRole("img", { name: "Idle" })).toBeVisible();
    await expect(canvas.getByRole("textbox", { name: "Tell the Commander" })).toBeVisible();
    await expect(canvas.queryByRole("listitem")).toBeNull();
    await expect(canvas.queryByRole("note")).toBeNull();
    await userEvent.click(canvas.getByRole("button", { name: "Hide Commander" }));
    await expect(args.onHide).toHaveBeenCalled();
  },
};

/** A turn Fleet started, with what the Commander read, did on its own and said: each row is named by its mark. */
export const WokenTurnWithActs: Story = {
  args: { rows: WOKEN },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("img", { name: "Fleet woke it" })).toBeVisible();
    await expect(canvas.getByText("Check store failed")).toBeVisible();
    await expect(canvas.getByRole("img", { name: "Raised cap" })).toBeVisible();
    await expect(canvas.getByText("Turn cap 40 → 60")).toBeVisible();
    await expect(canvas.getByText("The Drone ran out of turns on the third retry.")).toBeVisible();
  },
};

function Asking(props: CommanderPanelProps) {
  const [rows, setRows] = useState<readonly SessionThreadRow[]>(props.rows);
  return (
    <CommanderPanel
      {...props}
      rows={rows}
      onSend={(text) => {
        props.onSend(text);
        setRows((was) => [...was, { id: `you${String(was.length)}`, at: "14:31:00", kind: "message", from: "you", text }]);
      }}
    />
  );
}

/** Asking and reading the answer is one loop: what is sent lands in the thread, the box empties, and a blank sends nothing. */
export const OwnerMessage: Story = {
  args: { jobId: "job-owner", rows: WOKEN },
  render: (args) => <Asking {...args} />,
  play: async ({ canvas, args }) => {
    const send = canvas.getByRole("button", { name: "Send" });
    await expect(send).toBeDisabled();
    const box = canvas.getByRole("textbox", { name: "Tell the Commander" });
    await userEvent.type(box, "   ");
    await expect(send).toBeDisabled();
    await userEvent.clear(box);
    await userEvent.type(box, "  Raise the cost cap too  ");
    await userEvent.click(send);
    await expect(args.onSend).toHaveBeenCalledTimes(1);
    await expect(args.onSend).toHaveBeenCalledWith("Raise the cost cap too");
    await expect(box).toHaveValue("");
    await expect(canvas.getByText("Raise the cost cap too")).toBeVisible();
    await expect(send).toBeDisabled();
  },
};

/** A turn is out: the band and the thread's end both carry the working mark. */
export const Working: Story = {
  args: { state: "working", rows: WOKEN.slice(0, 2) },
  play: async ({ canvas }) => {
    await expect(canvas.getAllByRole("img", { name: "Working" })).toHaveLength(2);
    await expect(canvas.queryByRole("img", { name: "Idle" })).toBeNull();
  },
};

/** The Commander is held on the owner. */
export const WaitingOnYou: Story = {
  args: { state: "waiting", rows: WOKEN },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("img", { name: "Waiting on you" })).toBeVisible();
  },
};

function Panels() {
  const [shown, setShown] = useState(true);
  const [jobId, setJobId] = useState("job-draft-a");
  return (
    <div style={{ display: "flex", flexDirection: "column", width: "100%" }}>
      <div>
        <button onClick={() => setShown((was) => !was)}>Toggle</button>
        <button onClick={() => setJobId("job-draft-b")}>Other Job</button>
      </div>
      {shown ? <CommanderPanel jobId={jobId} state="idle" rows={[]} onSend={fn()} /> : null}
    </div>
  );
}

/** What was typed and not sent is still there on coming back to the Job, and another Job starts empty. */
export const KeepsADraft: Story = {
  render: () => <Panels />,
  play: async ({ canvas }) => {
    const box = () => canvas.getByRole("textbox", { name: "Tell the Commander" });
    await userEvent.type(box(), "half a thought");
    await userEvent.click(canvas.getByRole("button", { name: "Toggle" }));
    await expect(canvas.queryByRole("textbox", { name: "Tell the Commander" })).toBeNull();
    await userEvent.click(canvas.getByRole("button", { name: "Toggle" }));
    await expect(box()).toHaveValue("half a thought");
    await userEvent.click(canvas.getByRole("button", { name: "Other Job" }));
    await userEvent.click(canvas.getByRole("button", { name: "Toggle" }));
    await userEvent.click(canvas.getByRole("button", { name: "Toggle" }));
    await expect(box()).toHaveValue("");
  },
};
