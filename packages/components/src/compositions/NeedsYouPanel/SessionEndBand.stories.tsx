import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import { expect, fn, userEvent, waitFor } from "storybook/test";

import type { LedgerEntry } from "../SessionLedger/SessionLedger";
import { SessionEndBand } from "./SessionEndBand";

/** The agent's last word as a band on the composer: folded until a press, then what was made and the two ways on. */
const meta: Meta<typeof SessionEndBand> = {
  title: "Compositions/Session end band",
  component: SessionEndBand,
};
export default meta;

type Story = StoryObj<typeof SessionEndBand>;

const end = fn();
const keep = fn();
const made: LedgerEntry[] = [
  { key: "p1", kind: "pull_request", name: "Pull request #1843", text: "#1843 Pin the store clock", pr: { merged: true, checks: "passed", checksSaid: "Checks passed" }, onOpen: fn() },
  { key: "p2", kind: "pull_request", name: "Pull request #1850", text: "#1850 Name the store migrations", pr: { merged: false, checks: "passed", checksSaid: "Checks passed" }, onOpen: fn() },
];

function Frame() {
  const [open, setOpen] = useState(false);
  return (
    <div className="armada-session-frame__centre" style={{ display: "flex", flexDirection: "column", height: "calc(var(--space-12) * 8)", width: "calc(var(--space-12) * 10)" }}>
      <div className="armada-session-thread__stage" data-testid="stage" style={{ position: "relative", flex: 1 }}>
        <p>The thread</p>
      </div>
      <div style={{ position: "relative", marginBlockStart: "var(--h-badge)" }}>
        <SessionEndBand done={{ onEnd: end, onKeep: keep, made }} open={open} onOpen={setOpen} />
        <textarea aria-label="Message" />
      </div>
    </div>
  );
}

/** Folded, it is a band on the composer and nothing of the made list or the acts is there; it does not open by itself. */
export const FoldedUntilPressed: Story = {
  render: () => <Frame />,
  play: async ({ canvas }) => {
    const band = canvas.getByRole("button", { name: "Nothing left to do" });
    await expect(canvas.getByRole("region", { name: "Nothing left to do" })).toBeInTheDocument();
    await expect(band).toHaveAttribute("aria-expanded", "false");
    await new Promise((later) => setTimeout(later, 300));
    await expect(band).toHaveAttribute("aria-expanded", "false");
    await expect(canvas.queryByRole("button", { name: "End job" })).toBeNull();
    await expect(canvas.queryByRole("list", { name: "Pull requests" })).toBeNull();
  },
};

/** A press opens what was made and the two ways on; End job and Keep going each do their one thing. */
export const APressOpensWhatWasMade: Story = {
  render: () => <Frame />,
  play: async ({ canvas }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Nothing left to do" }));
    await expect(canvas.getByRole("list", { name: "Pull requests" })).toBeVisible();
    await expect(canvas.getByRole("button", { name: "Open Pull request #1843" }).closest("li")).not.toBeNull();
    await userEvent.click(canvas.getByRole("button", { name: "End job" }));
    await expect(end).toHaveBeenCalledTimes(1);
    await userEvent.click(canvas.getByRole("button", { name: "Keep going" }));
    await expect(keep).toHaveBeenCalledTimes(1);
  },
};

/** Esc and a press on the dimmed thread each fold it back to the band. */
export const EscAndThePressOnTheThreadFold: Story = {
  render: () => <Frame />,
  play: async ({ canvas }) => {
    const band = canvas.getByRole("button", { name: "Nothing left to do" });
    await userEvent.click(band);
    await expect(band).toHaveAttribute("aria-expanded", "true");
    canvas.getByRole("button", { name: "End job" }).focus();
    await userEvent.keyboard("{Escape}");
    await waitFor(() => expect(band).toHaveAttribute("aria-expanded", "false"));
    await userEvent.click(band);
    await userEvent.click(canvas.getByTestId("stage"));
    await waitFor(() => expect(band).toHaveAttribute("aria-expanded", "false"));
  },
};

/** With focus outside the band, Esc still folds it. */
export const EscFoldsWithFocusOutside: Story = {
  render: () => <Frame />,
  play: async ({ canvas }) => {
    const band = canvas.getByRole("button", { name: "Nothing left to do" });
    await userEvent.click(band);
    await expect(band).toHaveAttribute("aria-expanded", "true");
    canvas.getByRole("textbox", { name: "Message" }).focus();
    await userEvent.keyboard("{Escape}");
    await waitFor(() => expect(band).toHaveAttribute("aria-expanded", "false"));
  },
};
