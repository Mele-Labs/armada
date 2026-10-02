import type { Meta, StoryObj } from "@storybook/react-vite";
import { MessageSquare } from "lucide-react";
import { useState } from "react";
import { expect, fireEvent, fn, waitFor, within } from "storybook/test";
import { Button } from "../Button/Button";
import { Sheet } from "./Sheet";

const meta: Meta<typeof Sheet> = {
  title: "Primitives/Sheet",
  component: Sheet,
};
export default meta;

type Story = StoryObj<typeof Sheet>;

/**
 * The contract names no Sheet state. It names one surface treatment and no
 * side, no width, no use. This story exists so the surface treatment can be
 * seen; what a sheet is for on Bridge is in the report as an open item — the
 * layout model says full-width routes with no inspector pane and no modal for
 * job detail, which rules out the two things a sheet usually does.
 */
export const Right: Story = {
  args: {
    kind: "story-sheet",
    open: true,
    side: "right",
    title: "Kit allowlist",
    children:
      "The command tripped the allowlist 5 times and was approved every time. Adding it here stops the prompt.",
    onClose: fn(),
  },
  /**
   * **Two exits and no third**, which is a rule about what does *not* happen
   * and so is the kind only a run can hold. A dialog's scrim usually closes
   * it; this one takes no press, because a 1676-entry read must not be lost to
   * a stray click beside the panel.
   *
   * The negative assertion is the valuable half. Adding an `onClick` to the
   * scrim is a one-line change that looks like a fix and would render
   * identically — nothing else here would notice.
   */
  play: async ({ args, canvas, userEvent }) => {
    const panel = canvas.getByRole("dialog", { name: "Kit allowlist" });

    // The ground behind, reached through the structure rather than by class
    // name: the scrim is what the panel renders inside, and it carries no role
    // on purpose because it is not a control.
    const ground = panel.parentElement;
    if (ground === null) throw new Error("the sheet renders inside its scrim");

    // Dispatched rather than clicked. A pointer press has to land somewhere,
    // and every point on the scrim is either the panel or bare ground whose
    // position depends on the window — this is the press the scrim would have
    // to handle, put straight on it.
    fireEvent.click(ground);
    await expect(args.onClose).not.toHaveBeenCalled();

    // The second exit. The first is the close control, which is a button and
    // needs no assertion to prove it is one.
    await userEvent.keyboard("{Escape}");
    await expect(args.onClose).toHaveBeenCalled();
  },
};

export const Left: Story = {
  args: {
    kind: "story-sheet",
    open: true,
    side: "left",
    title: "Kit allowlist",
    children:
      "The command tripped the allowlist 5 times and was approved every time. Adding it here stops the prompt.",
  },
};

/**
 * Over the whole work area, under the title row and held off every edge, with
 * the card's radius and edge: Helm's dock, as a sheet. What the Record's open
 * row takes, where a sheet contained in the content column read as cut off at
 * that column's edge.
 */
export const Floating: Story = {
  args: {
    kind: "story-sheet",
    open: true,
    floating: true,
    title: "screens_test",
    closeLabel: "Close",
    closeBinding: "Esc",
    children: "AssertionError: expected 'board' to be 'job'",
  },
};

/**
 * The head's leading slot. Its one caller is Helm's folded dock (#1320), and
 * what it carries is that dock's own chip — the class below is `TheShell`'s,
 * not this primitive's, since the slot owns where a mark sits and never what
 * the mark is.
 *
 * `leading` is decoration and adds nothing to the dialog's name: the sheet is
 * still found by its title alone, which is what the play reads.
 */
export const Leading: Story = {
  args: {
    kind: "story-sheet",
    open: true,
    title: "Helm",
    leading: (
      <span className="armada-shell__dock-chip" aria-hidden>
        <MessageSquare size={16} strokeWidth={2} />
      </span>
    ),
    closeLabel: "Close",
    children: "Questions waiting on you, and Helm, will be here.",
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("dialog", { name: "Helm" })).toBeVisible();
  },
};

/**
 * **The way back**, where a press in another panel opened this one — Plan's
 * task panel sending a person to its Drone. It takes the head's first line
 * with the close, so the title keeps the width under it.
 */
export const Back: Story = {
  args: {
    kind: "story-sheet",
    open: true,
    floating: true,
    size: "wide",
    title: "Drone on T6",
    subtitle: "Implement · T6 · running",
    back: { label: "Back to T6", tooltip: "Back to Plan · T6", binding: "⌘[", onBack: fn() },
    closeLabel: "Close",
    closeBinding: "Esc",
    children: "The Drone's turns.",
    onClose: fn(),
  },
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: /Back to T6/ }));
    await expect(args.back?.onBack).toHaveBeenCalled();
    await expect(args.onClose).not.toHaveBeenCalled();
  },
};

/** Two kinds of sheet, opened one at a time: a contained log and a floating task panel. */
function TwoKinds() {
  const [open, setOpen] = useState<"log" | "task" | null>(null);
  const close = () => setOpen(null);
  return (
    // Wider than the test's viewport, so both sheets have room to drag, and
    // `contain` lays the floating one out against this frame as against a window.
    <div
      style={{
        position: "relative",
        contain: "layout",
        width: "calc(var(--w-dock) * 4)",
        height: "var(--palette-max-height)",
        background: "var(--bg-base)",
      }}
    >
      <Button variant="secondary" size="sm" onClick={() => setOpen("log")}>
        Open the log
      </Button>
      <Button variant="secondary" size="sm" onClick={() => setOpen("task")}>
        Open the task
      </Button>
      <Sheet open={open === "log"} kind="story-log" contained size="wide" title="Job log" closeLabel="Close" onClose={close}>
        The Job&apos;s own log.
      </Sheet>
      <Sheet open={open === "task"} kind="story-task" floating size="dock" title="T5" closeLabel="Close" onClose={close}>
        Draw what is running, in four lists.
      </Sheet>
    </div>
  );
}

/**
 * **Every sheet resizes, and each kind keeps its own width** (owner, 2 Oct
 * 2026). A contained sheet and a floating one each drag by their inner edge;
 * the other kind still opens at its own width, and a kind reopened opens at
 * the width it was dragged to.
 */
export const EachKindKeepsItsWidth: Story = {
  render: () => <TwoKinds />,
  play: async ({ canvas, userEvent }) => {
    window.localStorage.removeItem("armada.bridge.sheet-width");
    const open = (which: string) => userEvent.click(canvas.getByRole("button", { name: `Open the ${which}` }));
    const close = (title: string) =>
      userEvent.click(within(canvas.getByRole("dialog", { name: title })).getByRole("button", { name: "Close" }));
    const handle = (title: string) => canvas.getByRole("separator", { name: `Resize ${title}` });
    const now = (title: string) => Number(handle(title).getAttribute("aria-valuenow"));
    const width = async (title: string) => {
      await waitFor(() => expect(now(title)).toBeGreaterThan(0));
      return now(title);
    };
    // Toward the content widens a sheet on the trailing edge.
    const drag = (title: string, by: number) => {
      const grip = handle(title);
      const box = grip.getBoundingClientRect();
      const at = { pointerId: 1, button: 0, clientY: box.y + box.height / 2 };
      fireEvent.pointerDown(grip, { ...at, clientX: box.x });
      fireEvent.pointerMove(grip, { ...at, clientX: box.x - by });
      fireEvent.pointerUp(grip, { ...at, clientX: box.x - by });
    };

    await open("task");
    const taskAtRest = await width("T5");
    await close("T5");

    await open("log");
    const logAtRest = await width("Job log");
    drag("Job log", 120);
    await waitFor(() => expect(now("Job log")).toBe(logAtRest + 120));
    await close("Job log");

    // Another kind: the log's drag did not reach it.
    await open("task");
    await expect(await width("T5")).toBe(taskAtRest);
    drag("T5", 64);
    await waitFor(() => expect(now("T5")).toBe(taskAtRest + 64));
    await close("T5");

    // The same kind, reopened: where it was left, and the task's drag did not reach it.
    await open("log");
    await expect(await width("Job log")).toBe(logAtRest + 120);
    await close("Job log");
    await open("task");
    await expect(await width("T5")).toBe(taskAtRest + 64);
  },
};
